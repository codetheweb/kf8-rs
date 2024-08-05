use crate::serialization::tag_map::{TagDefinition, TagMapEntry, END_TAG_DEFINITION};

use super::types::{IndexTagMapEntry, TagMapEntryParseError};
#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ChunkTagMapEntry {
    // todo: rename?
    pub insert_position: u32,

    pub cncx_offset: u32,
    pub file_number: u32,
    pub sequence_number: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> TryFrom<&'a TagMapEntry> for ChunkTagMapEntry {
    type Error = TagMapEntryParseError;

    fn try_from(entry: &TagMapEntry) -> Result<Self, Self::Error> {
        let insert_position: u32 = entry
            .text
            .parse()
            .map_err(|_| TagMapEntryParseError::ParseError)?;

        let cncx_offset = entry
            .tag_map
            .get(&2)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("cncx_offset".to_string()))?[0];
        let file_number = entry
            .tag_map
            .get(&3)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("file_number".to_string()))?[0];
        let sequence_number = entry
            .tag_map
            .get(&4)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("sequence_number".to_string()))?[0];
        let geometry_pair = entry
            .tag_map
            .get(&6)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(ChunkTagMapEntry {
            insert_position,
            cncx_offset,
            file_number,
            sequence_number,
            start_offset,
            length,
        })
    }
}

impl Into<TagMapEntry> for ChunkTagMapEntry {
    fn into(self) -> TagMapEntry {
        let mut entry = TagMapEntry::default();
        entry.text = format!("{:010}", self.insert_position);
        entry.tag_map.insert(2, vec![self.cncx_offset]);
        entry.tag_map.insert(3, vec![self.file_number]);
        entry.tag_map.insert(4, vec![self.sequence_number]);
        entry.tag_map.insert(6, vec![self.start_offset, self.length]);
        entry
    }
}

impl<'a> IndexTagMapEntry<'a> for ChunkTagMapEntry {
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
        fn test_chunk_entry_roundtrip(entry in any::<super::ChunkTagMapEntry>()) {
            let downcasted_entry: TagMapEntry = entry.clone().into();

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            downcasted_entry.to_writer(&mut writer, &ChunkTagMapEntry::get_tag_definitions()).unwrap();
            writer.finalize().unwrap();

            serialized.set_position(0);
            let len = serialized.get_ref().len();
            let mut reader = Reader::new(&mut serialized);
            let decoded = TagMapEntry::from_reader_with_ctx(&mut reader, (len, &ChunkTagMapEntry::get_tag_definitions())).unwrap();
            let decoded = ChunkTagMapEntry::try_from(&decoded).unwrap();

            assert_eq!(entry, decoded);
        }
    }
}
