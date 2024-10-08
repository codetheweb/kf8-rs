use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use thiserror::Error;

use super::write_entry::MASK_TO_BIT_SHIFTS;

#[derive(Debug, Error)]
pub enum TagDefinitionConstructionError {
    #[error("Tag {0} is reserved")]
    ReservedTag(u8),
    #[error("Values per entry {0} is not a power of two")]
    ValuesPerEntryNotPowerOfTwo(u8),
    #[error("Mask {0} is unknown")]
    UnknownMask(u8),
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
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
    pub end_flag: u8,
    _private: (),
}

impl TagDefinition {
    pub fn new(
        tag: u8,
        values_per_entry: u8,
        mask: u8,
    ) -> Result<Self, TagDefinitionConstructionError> {
        if tag == 0 {
            return Err(TagDefinitionConstructionError::ReservedTag(tag));
        }

        if values_per_entry.count_ones() != 1 {
            return Err(TagDefinitionConstructionError::ValuesPerEntryNotPowerOfTwo(
                values_per_entry,
            ));
        }

        if !MASK_TO_BIT_SHIFTS.contains_key(&mask) {
            return Err(TagDefinitionConstructionError::UnknownMask(mask));
        }

        Ok(Self {
            tag,
            values_per_entry,
            mask,
            end_flag: 0,
            _private: (),
        })
    }
}

pub const END_TAG_DEFINITION: TagDefinition = TagDefinition {
    tag: 0,
    values_per_entry: 0,
    mask: 0,
    end_flag: 1,
    _private: (),
};
