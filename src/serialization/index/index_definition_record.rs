use crate::serialization::TagMapDefinition;
use deku::prelude::*;

#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(magic = b"INDX", endian = "big")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct IndexDefinitionRecord {
    pub len: u32, // todo: populate with length
    #[deku(temp, temp_value = "[0; 8]")]
    _unused0: [u8; 8],
    #[deku(temp, temp_value = "2")]
    _unused_header_type: u32,
    pub idxt_offset: u32,
    pub num_of_records: u32,
    #[deku(temp, temp_value = "65001")]
    _unused_encoding: u32,
    #[deku(temp, temp_value = "u32::MAX")]
    _unused1: u32,
    pub num_of_entries: u32,
    pub ordt_offset: u32,
    pub ligt_offset: u32,
    pub num_of_ordt_ligt_entries: u32,
    pub num_of_cncx_records: u32,
    #[deku(temp, temp_value = "[0; 124]")]
    _unused2: [u8; 124],
    pub tagx_offset: u32,
    #[deku(temp, temp_value = "[0; 8]")]
    _unused3: [u8; 8],
    pub definition: TagMapDefinition,
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
