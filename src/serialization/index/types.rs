use std::{collections::HashMap, io::Read, io::Write};

use cookie_factory::gen;
use deku::prelude::*;
use nom::{bytes::complete::take, error::ErrorKind, number::complete::be_u8, IResult};
use thiserror::Error;

use crate::{
    serialization::{serialize_tag_map, TagDefinition},
    tag_map::parse_tag_map,
};

#[derive(Debug, PartialEq)]
pub struct TagTableRow {
    pub text: String,
    pub tag_map: HashMap<u8, Vec<u32>>,
}

impl Default for TagTableRow {
    fn default() -> Self {
        TagTableRow {
            text: "".to_string(),
            tag_map: HashMap::new(),
        }
    }
}

fn read_tag_table_row<'a>(
    input: &'a [u8],
    table_definition: &Vec<TagDefinition>,
) -> IResult<&'a [u8], TagTableRow> {
    let (input, text_length) = be_u8(input)?;
    let (input, text) = take(text_length as usize)(input)?;
    // remap error
    let text = String::from_utf8(text.to_vec())
        .map_err(|_| nom::Err::Error(nom::error::make_error(input, ErrorKind::IsNot)))?;

    let (_, tag_map) = parse_tag_map(table_definition, input)?;

    Ok((input, TagTableRow { text, tag_map }))
}

// ctx is byte length of the row
// todo: move TagDefinition?
impl<'a> DekuReader<'a, (usize, &Vec<TagDefinition>)> for TagTableRow {
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        ctx: (usize, &Vec<TagDefinition>),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let (length, table_definition) = ctx;
        let mut buf = vec![0; length];
        reader.read_bytes(length, &mut buf)?;
        let (_, row) = read_tag_table_row(&buf, table_definition).unwrap();
        Ok(row)
    }
}

impl DekuWriter<&Vec<TagDefinition>> for TagTableRow {
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        definitions: &Vec<TagDefinition>,
    ) -> Result<(), DekuError> {
        writer.write_bytes(&[self.text.len() as u8])?;
        writer.write_bytes(self.text.as_bytes())?;

        let mut buf = Vec::new();
        gen(serialize_tag_map(definitions, &self.tag_map), &mut buf).unwrap();
        writer.write_bytes(&buf)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TagTableRowParseError {
    #[error("Tag {0} not found in table")]
    TagNotFound(String),
    #[error("Error parsing tag value")]
    ParseError,
}

pub trait IndexRow<'a>:
    TryFrom<&'a TagTableRow, Error = TagTableRowParseError> + Into<TagTableRow> + Clone
{
    fn get_tag_definitions() -> Vec<TagDefinition>;
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::serialization::{SkeletonIndexRow, END_TAG_DEFINITION, MASK_TO_BIT_SHIFTS};

    use super::*;
    use prop::sample::SizeRange;
    use proptest::prelude::*;

    fn arbitrary_definitions(
        num_definitions: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Vec<TagDefinition>> {
        let possible_masks = MASK_TO_BIT_SHIFTS
            .keys()
            // .filter(|m| m.count_ones() == 1)
            .copied()
            .collect::<Vec<u8>>();

        // 0 is reserved for end flag
        let possible_tags = (1..=u8::MAX).collect::<Vec<u8>>();
        let num_definitions: SizeRange = num_definitions.into();
        let num_definitions =
            num_definitions.start()..num_definitions.end_excl().min(possible_masks.len());
        let tags =
            proptest::sample::subsequence(possible_tags.clone(), num_definitions).prop_shuffle();

        tags.prop_ind_flat_map(move |tags| {
            let values_per_entry_power = proptest::collection::vec(0..=7u8, tags.len());
            let masks =
                proptest::sample::subsequence(possible_masks.clone(), tags.len()).prop_shuffle();

            (Just(tags), (values_per_entry_power, masks))
        })
        .prop_map(move |(tags, (values_per_entry_power, masks))| {
            let mut definitions = Vec::new();
            for i in 0..tags.len() {
                definitions.push(
                    TagDefinition::new(
                        tags[i],
                        2u8.pow(values_per_entry_power[i].into()),
                        masks[i],
                    )
                    .unwrap(),
                );
            }

            definitions.push(END_TAG_DEFINITION);
            definitions
        })
    }

    fn arbitrary_row(
        definitions: impl Strategy<Value = Vec<TagDefinition>> + Clone,
    ) -> impl Strategy<Value = TagTableRow> {
        let definitions = definitions.prop_map(|definitions| {
            definitions
                .iter()
                // Remove end flag definition
                .filter(|d| d.tag != 0)
                .cloned()
                .collect::<Vec<TagDefinition>>()
        });

        let value_count_multipliers = definitions
            .clone()
            .prop_ind_flat_map(|definitions| {
                let len = definitions.len();
                (
                    Just(definitions),
                    proptest::collection::vec(any::<bool>(), len),
                )
            })
            .prop_map(|(definitions, is_value_count_multiplied)| {
                println!("is_value_count_multiplied: {:?}", is_value_count_multiplied);
                definitions
                    .iter()
                    .zip(is_value_count_multiplied)
                    .map(|(d, is_multiplied)| {
                        if is_multiplied && d.mask.count_ones() == 2 {
                            println!("multiplied: {}", d.mask);
                            // return d.mask.count_ones() as usize;
                            return 2;
                        }

                        1
                    })
                    .collect::<Vec<usize>>()
            });

        // This is a little weird (contiguous array) because proptest doesn't support Vec<impl Strategy> -> Strategy<Vec> yet
        let maps = (definitions, value_count_multipliers)
            .prop_flat_map(|(definitions, value_count_multipliers)| {
                let total_num_values = definitions
                    .iter()
                    .zip(value_count_multipliers.clone())
                    .map(|(d, multiplier)| d.values_per_entry as usize * multiplier)
                    .sum::<usize>();

                (
                    (Just(definitions), Just(value_count_multipliers)),
                    proptest::collection::vec(0u32..=u32::MAX, total_num_values),
                )
            })
            .prop_map(|((definitions, value_count_multipliers), mut values)| {
                println!("multipliers: {:?}", value_count_multipliers);
                let mut tag_map = HashMap::new();
                for (definition, value_count_multiplier) in
                    definitions.iter().zip(value_count_multipliers)
                {
                    let v = values
                        .drain(0..definition.values_per_entry as usize * value_count_multiplier)
                        .collect();
                    tag_map.insert(definition.tag, v);
                }

                tag_map
            });

        ("\\PC*", maps).prop_map(|(text, tag_map)| TagTableRow { text, tag_map })
    }

    fn arbitrary_definition_and_row() -> impl Strategy<Value = (Vec<TagDefinition>, TagTableRow)> {
        // Maximum number of definitions is limited by number of defined masks
        arbitrary_definitions(1..=100).prop_ind_flat_map(|definitions| {
            (
                Just(definitions.clone()),
                arbitrary_row(Just(definitions.clone())),
            )
        })
    }

    proptest! {
      #![proptest_config(ProptestConfig {
        max_shrink_iters: 100_000,
        // this logic is kinda tricky, so let's give it 4x the normal number of cases
        cases: 1024,
        .. ProptestConfig::default()
      })]

      #[test]
      fn test_tag_table_row_roundtrip((definitions, row) in arbitrary_definition_and_row()) {
        // println!("definitions:");
        let mut masks = MASK_TO_BIT_SHIFTS.keys().copied().collect::<Vec<_>>();
        masks.sort();
        // println!("definitions: {:?}", definitions);
        // for mask in masks {
        //     println!("mask: {:08b} ({})", mask, mask);
        // }

        env_logger::try_init();

        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        row.to_writer(&mut writer, &definitions).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let len = serialized.get_ref().len();
        let mut reader = Reader::new(&mut serialized);
        let decoded = TagTableRow::from_reader_with_ctx(&mut reader, (len, &definitions)).unwrap();

        assert_eq!(row, decoded);
      }
    }

    #[test]
    fn test_foo() {
        env_logger::try_init();

        let definitions = vec![
            TagDefinition::new(102, 8, 8).unwrap(),
            TagDefinition::new(186, 8, 32).unwrap(),
            TagDefinition::new(234, 16, 48).unwrap(),
            TagDefinition::new(60, 128, 2).unwrap(),
            TagDefinition::new(193, 4, 4).unwrap(),
            TagDefinition::new(10, 32, 12).unwrap(),
            END_TAG_DEFINITION,
        ];

        let mut row = TagTableRow::default();

        row.tag_map.insert(102, vec![0; 8]);
        row.tag_map.insert(186, vec![0; 8]);
        row.tag_map.insert(234, vec![0; 32]);
        row.tag_map.insert(60, vec![0; 128]);
        row.tag_map.insert(193, vec![0; 4]);
        row.tag_map.insert(10, vec![0; 64]);

        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        row.to_writer(&mut writer, &definitions).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let len = serialized.get_ref().len();
        let mut reader = Reader::new(&mut serialized);
        let decoded = TagTableRow::from_reader_with_ctx(&mut reader, (len, &definitions)).unwrap();

        assert_eq!(row, decoded);
    }

    #[test]
    fn test_foo1() {
        let definitions = SkeletonIndexRow::get_tag_definitions();
        let row = SkeletonIndexRow {
            name: "foo".to_string(),
            chunk_count: 0,
            start_offset: 0,
            length: 0,
        };
        let row: TagTableRow = row.into();

        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        row.to_writer(&mut writer, &definitions).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let len = serialized.get_ref().len();
        let mut reader = Reader::new(&mut serialized);
        let decoded = TagTableRow::from_reader_with_ctx(&mut reader, (len, &definitions)).unwrap();

        assert_eq!(row, decoded);
    }
}
