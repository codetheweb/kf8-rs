use deku::prelude::*;
#[cfg(test)]
use proptest::{arbitrary::any, prop_compose};
#[cfg(test)]
use proptest_derive::Arbitrary;
use std::collections::HashMap;

use crate::{
    constants::{MainLanguage, MetadataId, MetadataIdValue, SubLanguage},
    serialization::ExtraDataFlags,
    K8Header,
};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Codepage {
    #[deku(id = "0x000004e4")]
    Cp1252,
    #[deku(id = "0x0000fde9")]
    Utf8,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SectionHeader {
    pub offset: u32,
    pub flags: u8,
    #[cfg_attr(test, proptest(strategy = "0..(u32::from(ux::u24::MAX))"))]
    pub val: u32,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum MobiHeaderIdent {
    BookMobi,
    TextRead,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum DocType {
    Mobi,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
#[deku(id_type = "u16", ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub enum CompressionType {
    #[deku(id = 0x0001)]
    None,
    #[deku(id = 0x0002)]
    PalmDoc,
    #[deku(id = 0x4448)]
    HuffCdic,
}

#[derive(Debug, PartialEq)]
pub struct MobiHeader {
    pub name: String,
    // todo: should remove this field
    pub num_sections: u16,
    pub ident: MobiHeaderIdent,
    pub section_headers: Vec<SectionHeader>,
}

#[cfg(test)]
prop_compose! {
    // name is limited to valid ASCII characters rather than UTF-8 because otherwise the codepoint splitting gets weird
    pub fn mobi_header()(name in "[\x01-\x7F]{0,31}", ident in any::<MobiHeaderIdent>(), section_headers in any::<Vec<SectionHeader>>()) -> MobiHeader {
        MobiHeader {
            name,
            num_sections: section_headers.len() as u16,
            ident,
            section_headers
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BookHeader {
    pub compression_type: CompressionType,
    pub records: u16,
    pub records_size: u16,
    // todo: enum?
    pub encryption_type: u16,
    // todo: enum?
    pub doctype: DocType,
    pub unique_id: u32,
    pub language: Option<MainLanguage>,
    pub sub_language: Option<SubLanguage>,
    pub ncxidx: u32,
    pub extra_flags: ExtraDataFlags,
    pub k8: Option<K8Header>,
    pub title: String,
    pub standard_metadata: Option<HashMap<MetadataId, Vec<String>>>,
    pub kf8_metadata: Option<HashMap<MetadataIdValue, Vec<u32>>>,
    pub first_resource_section_index: usize,
}

impl BookHeader {
    pub fn sizeof_trailing_section_entries(&self, section_data: &[u8]) -> usize {
        let mut num = 0;
        let size = section_data.len();

        fn sizeof_trailing_section_entry(section_data: &[u8], offset: usize) -> usize {
            let mut offset = offset;
            let mut bitpos = 0;
            let mut result: usize = 0;

            loop {
                let v = section_data[offset - 1] as usize;
                result |= (v & 0x7f) << bitpos;
                bitpos += 7;
                offset -= 1;

                if (v & 0x80) != 0 || (bitpos >= 28) || offset == 0 {
                    return result;
                }
            }
        }

        let mut encoded_flags = self.extra_flags.encode() >> 1;

        while encoded_flags > 0 {
            if encoded_flags & 1 > 0 {
                num += sizeof_trailing_section_entry(section_data, size - num);
            }

            encoded_flags >>= 1;
        }

        if self.extra_flags.extra_multibyte_bytes_after_text_records {
            let offset = size - num - 1;
            num += (section_data[offset] as usize & 0x3) + 1;
        }

        num
    }

    pub fn get_bcp47_language_tag(&self) -> Option<&'static str> {
        return self
            .sub_language
            .as_ref()
            .map_or(self.language.as_ref().map(|l| l.to_bcp47()), |l| {
                Some(l.to_bcp47())
            });
    }
}
