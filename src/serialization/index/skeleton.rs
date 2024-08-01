use crate::serialization::{TagDefinition, END_TAG_DEFINITION};

use super::types::{IndexRow, TagTableRow, TagTableRowParseError};

#[derive(Debug, PartialEq)]
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
            TagDefinition {
                tag: 1,
                values_per_entry: 1,
                mask: 3,
                end_flag: 0,
            },
            TagDefinition {
                tag: 6,
                values_per_entry: 2,
                mask: 12,
                end_flag: 0,
            },
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
