#[cfg(test)]
use crate::serialization::MASK_TO_BIT_SHIFTS;
use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
// todo: remove arbitrary here?
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct TagDefinition {
    #[cfg_attr(test, proptest(strategy = "1u8..=u8::MAX"))] // 0 is reserved for end flag
    pub tag: u8,
    #[cfg_attr(test, proptest(strategy = "1u8..=10u8"))]
    pub values_per_entry: u8,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::sample::select(MASK_TO_BIT_SHIFTS.keys().copied().collect::<Vec<u8>>())"
        )
    )]
    pub mask: u8,
    #[cfg_attr(test, proptest(strategy = "0u8..=0u8"))]
    pub end_flag: u8,
}

pub const END_TAG_DEFINITION: TagDefinition = TagDefinition {
    tag: 0,
    values_per_entry: 0,
    mask: 0,
    end_flag: 1,
};

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
