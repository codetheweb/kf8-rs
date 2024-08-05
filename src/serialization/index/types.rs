use crate::serialization::tag_map::{TagDefinition, TagMapEntry};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TagMapEntryParseError {
    #[error("Tag {0} not found in map")]
    TagNotFound(String),
    #[error("Error parsing tag value")]
    ParseError,
}

pub trait IndexTagMapEntry<'a>:
    TryFrom<&'a TagMapEntry, Error = TagMapEntryParseError> + Into<TagMapEntry> + Clone
{
    fn get_tag_definitions() -> Vec<TagDefinition>;
}
