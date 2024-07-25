use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct TagDefinition {
    pub tag: u8,
    pub values_per_entry: u8,
    pub mask: u8,
    pub end_flag: u8,
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big", magic = b"TAGX")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct TagTableDefinition {
    #[deku(temp, temp_value = "((tag_definitions.len() * 4) + 12) as u32")]
    _len: u32,
    #[deku(temp, temp_value = "1")]
    _control_byte_count: u32,
    #[deku(count = "(_len - 12) / 4")]
    pub tag_definitions: Vec<TagDefinition>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_tag_section_roundtrip(section in any::<TagTableDefinition>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        section.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = TagTableDefinition::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(section, decoded);
      }
    }
}
