use crate::serialization::{IndxHeader, TagTableDefinition};
use deku::prelude::*;

#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct IndexDefinitionRecord {
    pub header: IndxHeader,
    pub definition: TagTableDefinition,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_index_definition_record_roundtrip(record in any::<IndexDefinitionRecord>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        record.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = IndexDefinitionRecord::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(record, decoded);
      }
    }
}
