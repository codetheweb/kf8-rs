use super::types::{IndexRow, TagTableRow, TagTableRowParseError};
use crate::serialization::{TagDefinition, END_TAG_DEFINITION};
#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ChunkIndexRow {
    // todo: rename?
    pub insert_position: u32,

    pub cncx_offset: u32,
    pub file_number: u32,
    pub sequence_number: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> TryFrom<&'a TagTableRow> for ChunkIndexRow {
    type Error = TagTableRowParseError;

    fn try_from(row: &TagTableRow) -> Result<Self, Self::Error> {
        let insert_position: u32 = row
            .text
            .parse()
            .map_err(|_| TagTableRowParseError::ParseError)?;

        let cncx_offset = row
            .tag_map
            .get(&2)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("cncx_offset".to_string()))?[0];
        let file_number = row
            .tag_map
            .get(&3)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("file_number".to_string()))?[0];
        let sequence_number = row
            .tag_map
            .get(&4)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("sequence_number".to_string()))?[0];
        let geometry_pair = row
            .tag_map
            .get(&6)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(ChunkIndexRow {
            insert_position,
            cncx_offset,
            file_number,
            sequence_number,
            start_offset,
            length,
        })
    }
}

impl Into<TagTableRow> for ChunkIndexRow {
    fn into(self) -> TagTableRow {
        let mut row = TagTableRow::default();
        row.text = format!("{:010}", self.insert_position);
        row.tag_map.insert(2, vec![self.cncx_offset]);
        row.tag_map.insert(3, vec![self.file_number]);
        row.tag_map.insert(4, vec![self.sequence_number]);
        row.tag_map.insert(6, vec![self.start_offset, self.length]);
        row
    }
}

impl<'a> IndexRow<'a> for ChunkIndexRow {
    fn get_tag_definitions() -> Vec<TagDefinition> {
        vec![
            TagDefinition::new(2, 1, 1).unwrap(),
            TagDefinition::new(3, 1, 2).unwrap(),
            TagDefinition::new(4, 1, 4).unwrap(),
            TagDefinition::new(6, 2, 8).unwrap(),
            END_TAG_DEFINITION,
        ]
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
        fn test_chunk_index_row_roundtrip(row in any::<super::ChunkIndexRow>()) {
            println!("testing row: {:?}", row);
            let table_row: TagTableRow = row.clone().into();

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            table_row.to_writer(&mut writer, &ChunkIndexRow::get_tag_definitions()).unwrap();
            writer.finalize().unwrap();

            serialized.set_position(0);
            let len = serialized.get_ref().len();
            let mut reader = Reader::new(&mut serialized);
            let decoded = TagTableRow::from_reader_with_ctx(&mut reader, (len, &ChunkIndexRow::get_tag_definitions())).unwrap();
            let decoded = ChunkIndexRow::try_from(&decoded).unwrap();

            assert_eq!(row, decoded);
        }
    }
}
