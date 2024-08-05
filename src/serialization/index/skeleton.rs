use crate::serialization::{TagDefinition, END_TAG_DEFINITION};
#[cfg(test)]
use proptest_derive::Arbitrary;

use super::types::{IndexRow, TagTableRow, TagTableRowParseError};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SkeletonIndexRow {
    pub name: String,
    pub chunk_count: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> IndexRow<'a> for SkeletonIndexRow {
    fn get_tag_definitions() -> Vec<TagDefinition> {
        // todo: lazy static
        vec![
            TagDefinition::new(1, 1, 3).unwrap(),
            TagDefinition::new(6, 2, 12).unwrap(),
            END_TAG_DEFINITION,
        ]
    }
}

impl<'a> TryFrom<&'a TagTableRow> for SkeletonIndexRow {
    type Error = TagTableRowParseError;

    fn try_from(row: &TagTableRow) -> Result<Self, Self::Error> {
        let chunk_count = row
            .tag_map
            .get(&1)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("chunk_count".to_string()))?[0];
        let geometry_pair = row
            .tag_map
            .get(&6)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(SkeletonIndexRow {
            name: row.text.clone(),
            chunk_count,
            start_offset,
            length,
        })
    }
}

impl Into<TagTableRow> for SkeletonIndexRow {
    fn into(self) -> TagTableRow {
        let mut row = TagTableRow::default();
        row.text = self.name;
        row.tag_map
            .insert(1, vec![self.chunk_count, self.chunk_count]);
        row.tag_map.insert(
            6,
            vec![
                self.start_offset,
                self.length,
                self.start_offset,
                self.length,
            ],
        );
        row
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use deku::{reader::Reader, writer::Writer, DekuReader, DekuWriter};
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
        #[test]
        fn test_skeleton_index_row_roundtrip(row in any::<super::SkeletonIndexRow>()) {
            println!("testing row: {:?}", row);
            let table_row: TagTableRow = row.clone().into();

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            table_row.to_writer(&mut writer, &SkeletonIndexRow::get_tag_definitions()).unwrap();
            writer.finalize().unwrap();

            serialized.set_position(0);
            let len = serialized.get_ref().len();
            let mut reader = Reader::new(&mut serialized);
            let decoded = TagTableRow::from_reader_with_ctx(&mut reader, (len, &SkeletonIndexRow::get_tag_definitions())).unwrap();
            let decoded = SkeletonIndexRow::try_from(&decoded).unwrap();

            assert_eq!(row, decoded);
        }
    }
}
