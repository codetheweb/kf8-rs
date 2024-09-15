use deku::prelude::*;

#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(magic = b"INDX", endian = "big")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct IndexMetaDefinitionRecord {
    #[deku(temp, temp_value = "192")]
    len: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused0: [u8; 4],
    #[deku(temp, temp_value = "1")]
    _unused_index_type: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused1: [u8; 4],
    pub idxt_block_offset: u32,
    pub num_index_entries: u32,
    #[deku(temp, temp_value = "[0xff; 8]")]
    _unused2: [u8; 8],
    #[deku(temp, temp_value = "[0; 156]")]
    _unused3: [u8; 156],
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_index_meta_definition_record_roundtrip(record in any::<IndexMetaDefinitionRecord>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        record.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = IndexMetaDefinitionRecord::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(record, decoded);
      }
    }
}
