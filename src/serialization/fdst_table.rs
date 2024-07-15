use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct FDSTEntry {
    pub start: u32,
    pub end: u32,
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big", magic = b"FDST")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct FDSTTable {
    #[deku(temp, temp_value = "12")]
    _unused_len: u32,
    #[deku(temp, temp_value = "entries.len() as u32")]
    num_entries: u32,
    #[deku(count = "num_entries")]
    pub entries: Vec<FDSTEntry>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_fdst_table_roundtrip(table in any::<FDSTTable>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        table.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = FDSTTable::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(table, decoded);
      }
    }
}
