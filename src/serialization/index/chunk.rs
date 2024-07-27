use super::types::{IndexRow, TagTableRow, TagTableRowParseError};
use crate::serialization::TagDefinition;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(Debug, PartialEq)]
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
            TagDefinition {
                tag: 2,
                values_per_entry: 1,
                mask: 1,
                end_flag: 0,
            },
            TagDefinition {
                tag: 3,
                values_per_entry: 1,
                mask: 2,
                end_flag: 0,
            },
            TagDefinition {
                tag: 4,
                values_per_entry: 1,
                mask: 4,
                end_flag: 0,
            },
            TagDefinition {
                tag: 6,
                values_per_entry: 2,
                mask: 8,
                end_flag: 0,
            },
        ]
    }
}
