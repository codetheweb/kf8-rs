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
    TryFrom<&'a TagTableRow, Error = TagTableRowParseError> + Into<TagTableRow>
{
    fn get_tag_definitions() -> Vec<TagDefinition>;
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::serialization::{END_TAG_DEFINITION, MASK_TO_BIT_SHIFTS};

    use super::*;
    use prop::sample::SizeRange;
    use proptest::prelude::*;

    fn arbitrary_definitions(
        num_definitions: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Vec<TagDefinition>> {
        // 0 is reserved for end flag
        let possible_tags = (1..=u8::MAX).collect::<Vec<u8>>();
        let tags = proptest::sample::subsequence(possible_tags.clone(), num_definitions);

        tags.prop_shuffle()
            .prop_flat_map(move |tags| {
                let values_per_entry = proptest::collection::vec(1u8..=10u8, tags.len());
                let mask = proptest::sample::select(
                    MASK_TO_BIT_SHIFTS.keys().copied().collect::<Vec<u8>>(),
                );

                (Just(tags), values_per_entry, mask)
            })
            .prop_flat_map(move |(tags, values_per_entry, mask)| {
                let mut definitions = Vec::new();
                for i in 0..tags.len() {
                    definitions.push(TagDefinition {
                        tag: tags[i],
                        values_per_entry: values_per_entry[i],
                        mask,
                        end_flag: 0,
                    });
                }

                definitions.push(END_TAG_DEFINITION);

                Just(definitions)
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

        // This is a little weird (contiguous array) because proptest doesn't support Vec<impl Strategy> -> Strategy<Vec> yet
        let values = definitions.clone().prop_flat_map(|definitions| {
            let total_num_values = definitions
                .iter()
                .map(|d| d.values_per_entry as usize)
                .sum::<usize>();

            proptest::collection::vec(0u32..=u32::MAX, total_num_values)
        });

        let maps = (definitions, values).prop_map(|(definitions, mut values)| {
            let mut tag_map = HashMap::new();
            for definition in definitions {
                let v = values
                    .drain(0..definition.values_per_entry as usize)
                    .collect();
                tag_map.insert(definition.tag, v);
            }

            tag_map
        });

        ("\\PC*", maps).prop_map(|(text, tag_map)| TagTableRow { text, tag_map })
    }

    fn arbitrary_definition_and_row() -> impl Strategy<Value = (Vec<TagDefinition>, TagTableRow)> {
        // todo: not one
        arbitrary_definitions(1).prop_flat_map(|definitions| {
            let row = arbitrary_row(Just(definitions.clone()));

            (Just(definitions), row)
        })
    }

    proptest! {
      #[test]
      fn test_tag_table_row_roundtrip((definitions, row) in arbitrary_definition_and_row()) {
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
}
