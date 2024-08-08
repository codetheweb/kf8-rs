use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

const INDX_HEADER_SIZE: usize = 36;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big", magic = b"INDX")]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct IndxHeader {
    // Something is really slow for large values of len
    #[cfg_attr(test, proptest(strategy = "INDX_HEADER_SIZE as u32..=10_000"))]
    pub len: u32, // todo: populate with length
    #[deku(temp, temp_value = "[0; 4]")]
    _unused0: [u8; 4],
    #[deku(temp, temp_value = "1")]
    _unused_header_type: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused1: [u8; 4],
    #[cfg_attr(test, proptest(filter = "|x| *x >= INDX_HEADER_SIZE as u32"))]
    pub block_offset: u32,
    pub num_entries: u32,
    #[deku(temp, temp_value = "[0xff; 8]")]
    _unused2: [u8; 8],
    #[deku(
        temp,
        count = "*len as usize - INDX_HEADER_SIZE",
        temp_value = "vec![0; *len as usize - INDX_HEADER_SIZE]"
    )]
    _unused3: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_indx_header_roundtrip(header in any::<IndxHeader>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        header.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = IndxHeader::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(header, decoded);
      }
    }
}
