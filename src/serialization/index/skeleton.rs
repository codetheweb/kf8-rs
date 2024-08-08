#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::serialization::tag_map::{TagDefinition, TagMapEntry, END_TAG_DEFINITION};

use super::types::{IndexTagMapEntry, TagMapEntryParseError};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SkeletonTagMapEntry {
    pub name: String,
    pub chunk_count: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> IndexTagMapEntry<'a> for SkeletonTagMapEntry {
    fn get_tag_definitions() -> Vec<TagDefinition> {
        // todo: lazy static
        vec![
            TagDefinition::new(1, 1, 3).unwrap(),
            TagDefinition::new(6, 2, 12).unwrap(),
            END_TAG_DEFINITION,
        ]
    }
}

impl<'a> TryFrom<&'a TagMapEntry> for SkeletonTagMapEntry {
    type Error = TagMapEntryParseError;

    fn try_from(entry: &TagMapEntry) -> Result<Self, Self::Error> {
        let chunk_count = entry
            .tag_map
            .get(&1)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("chunk_count".to_string()))?[0];
        let geometry_pair = entry
            .tag_map
            .get(&6)
            .ok_or_else(|| TagMapEntryParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(SkeletonTagMapEntry {
            name: entry.text.clone(),
            chunk_count,
            start_offset,
            length,
        })
    }
}

impl Into<TagMapEntry> for SkeletonTagMapEntry {
    fn into(self) -> TagMapEntry {
        let mut entry = TagMapEntry::default();
        entry.text = self.name;
        entry
            .tag_map
            .insert(1, vec![self.chunk_count, self.chunk_count]);
        entry.tag_map.insert(
            6,
            vec![
                self.start_offset,
                self.length,
                self.start_offset,
                self.length,
            ],
        );
        entry
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
        fn test_skeleton_entry_roundtrip(entry in any::<super::SkeletonTagMapEntry>()) {
            let downcasted_entry: TagMapEntry = entry.clone().into();

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            downcasted_entry.to_writer(&mut writer, &SkeletonTagMapEntry::get_tag_definitions()).unwrap();
            writer.finalize().unwrap();

            serialized.set_position(0);
            let len = serialized.get_ref().len();
            let mut reader = Reader::new(&mut serialized);
            let decoded = TagMapEntry::from_reader_with_ctx(&mut reader, (len, &SkeletonTagMapEntry::get_tag_definitions())).unwrap();
            let decoded = SkeletonTagMapEntry::try_from(&decoded).unwrap();

            assert_eq!(entry, decoded);
        }
    }
}
