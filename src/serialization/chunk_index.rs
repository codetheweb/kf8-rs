use std::collections::HashMap;

#[cfg(test)]
use proptest_derive::Arbitrary;
use thiserror::Error;

use super::TagDefinition;

pub struct TagTableRow {
    pub text: Option<String>,
    pub tag_map: HashMap<u8, Vec<u32>>,
}

impl Default for TagTableRow {
    fn default() -> Self {
        TagTableRow {
            text: None,
            tag_map: HashMap::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum TagTableRowParseError {
    #[error("Missing required text in table row")]
    MissingText,
    #[error("Tag {0} not found in table")]
    TagNotFound(String),
    #[error("Error parsing tag value")]
    ParseError,
}

trait Index: TryFrom<TagTableRow, Error = TagTableRowParseError> + Into<TagTableRow> {
    fn get_tag_definitions() -> Vec<TagDefinition>;
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ChunkIndex {
    // todo: rename?
    pub insert_position: u32,

    pub cncx_offset: u32,
    pub file_number: u32,
    pub sequence_number: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl TryFrom<TagTableRow> for ChunkIndex {
    type Error = TagTableRowParseError;

    fn try_from(row: TagTableRow) -> Result<Self, Self::Error> {
        let insert_position: u32 = row
            .text
            .ok_or_else(|| TagTableRowParseError::MissingText)?
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

        Ok(ChunkIndex {
            insert_position,
            cncx_offset,
            file_number,
            sequence_number,
            start_offset,
            length,
        })
    }
}

impl Into<TagTableRow> for ChunkIndex {
    fn into(self) -> TagTableRow {
        let mut row = TagTableRow::default();
        row.text = Some(format!("{:010}", self.insert_position));
        row.tag_map.insert(2, vec![self.cncx_offset]);
        row.tag_map.insert(3, vec![self.file_number]);
        row.tag_map.insert(4, vec![self.sequence_number]);
        row.tag_map.insert(6, vec![self.start_offset, self.length]);
        row
    }
}

impl Index for ChunkIndex {
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
