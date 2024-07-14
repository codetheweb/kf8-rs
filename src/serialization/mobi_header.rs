use std::io::{Read, Write};

use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::{
    constants::{MainLanguage, SubLanguage},
    types::{Codepage, CompressionType},
};

use super::exth::Exth;

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct LanguageCode {
    pub main: Option<MainLanguage>,
    pub sub: Option<SubLanguage>,
}

impl<'a, Ctx> DekuReader<'a, Ctx> for LanguageCode {
    fn from_reader_with_ctx<R: Read>(reader: &mut Reader<R>, ctx: Ctx) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let langcode = u32::from_reader_with_ctx(reader, ())?;

        let langid = langcode & 0xff;
        let sublangid = (langcode >> 10) & 0xff;

        // todo: don't unwrap
        let language = if langid == 0 {
            None
        } else {
            MainLanguage::try_from(langid).ok()
        };
        let sub_language = if sublangid == 0 {
            None
        } else {
            SubLanguage::try_from(sublangid).ok()
        };

        Ok(LanguageCode {
            main: language,
            sub: sub_language,
        })
    }
}

impl<Ctx> DekuWriter<Ctx> for LanguageCode {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _ctx: Ctx) -> Result<(), DekuError> {
        let language = self
            .main
            .clone()
            .map_or(0, |language| u8::from(language) as u32);
        let sub_language = self
            .sub
            .clone()
            .map_or(0, |sub_language| u8::from(sub_language) as u32);
        let langcode = (sub_language << 10) | language;

        langcode.to_writer(writer, ())
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ExtraDataFlags {
    pub extra_multibyte_bytes_after_text_records: bool,
    pub has_tbs: bool,
    pub uncrossable_breaks: bool,
}

impl ExtraDataFlags {
    pub fn encode(&self) -> u32 {
        let mut flags = 0;

        if self.extra_multibyte_bytes_after_text_records {
            flags |= 0b1;
        }

        if self.has_tbs {
            flags |= 0b10;
        }

        if self.uncrossable_breaks {
            flags |= 0b100;
        }

        flags
    }
}

impl<'a, Ctx> DekuReader<'a, Ctx> for ExtraDataFlags {
    fn from_reader_with_ctx<R: Read>(reader: &mut Reader<R>, ctx: Ctx) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let flags = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Big)?;

        Ok(ExtraDataFlags {
            extra_multibyte_bytes_after_text_records: (flags & 0b1) != 0,
            has_tbs: (flags & 0b10) != 0,
            uncrossable_breaks: (flags & 0b100) != 0,
        })
    }
}

impl<Ctx> DekuWriter<Ctx> for ExtraDataFlags {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _ctx: Ctx) -> Result<(), DekuError> {
        self.encode().to_writer(writer, deku::ctx::Endian::Big)
    }
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct MobiHeader {
    pub compression_type: CompressionType,
    #[deku(temp, temp_value = "[0x00; 2]")]
    _unused0: [u8; 2],
    pub text_length: u32,
    pub last_text_record: u16,
    pub text_record_size: u16,
    pub encryption_type: u16, // todo
    #[deku(temp, temp_value = "[0x00; 2]")]
    _unused1: [u8; 2],
    pub ident: [u8; 4], // todo
    #[deku(temp, temp_value = "264")]
    header_length: u32,
    pub book_type: u32, // todo
    pub text_encoding: Codepage,
    pub uid: u32,          // todo
    pub file_version: u32, // todo
    #[deku(temp, temp_value = "u32::MAX")]
    pub meta_orth_record: u32,
    #[deku(temp, temp_value = "u32::MAX")]
    pub meta_infl_index: u32,
    #[deku(temp, temp_value = "[u32::MAX; 8]")]
    pub extra_indices: [u32; 8],
    pub first_non_text_record: u32,
    pub title_offset: u32,           // todo: derive with deku?
    pub title_length: u32,           // todo: derive with deku?
    pub language_code: LanguageCode, // language_code
    pub in_lang: u32,                // todo
    pub out_lang: u32,               // todo
    pub min_version: u32,            // todo
    pub first_resource_record: u32,
    huff_first_record: u32,
    huff_count: u32,
    #[deku(temp, temp_value = "[u32::MAX; 4]")]
    huff_table_offset: [u32; 4],
    #[deku(temp, temp_value = "[u32::MAX; 4]")]
    huff_table_length: [u32; 4],
    pub exth_flags: u32, // todo
    #[deku(temp, temp_value = "[0x00; 8]")]
    _unused2: [u8; 8],
    #[deku(temp, temp_value = "u32::MAX")]
    _unused3: u32,
    #[deku(temp, temp_value = "u32::MAX")]
    drm_offset: u32,
    #[deku(temp, temp_value = "0")]
    drm_count: u32,
    #[deku(temp, temp_value = "0")]
    drm_size: u32,
    #[deku(temp, temp_value = "0")]
    drm_flags: u32,
    #[deku(temp, temp_value = "[0x00; 8]")]
    _unused4: [u8; 8],
    pub fdst_record: u32, // todo
    pub fdst_count: u32,  // todo
    pub fcis_record: u32, // todo
    // #[deku(assert_eq = "1")]
    pub fcis_count: u32,  // todo
    pub flis_record: u32, // todo
    // #[deku(assert_eq = "1")]
    pub flis_count: u32, // todo
    #[deku(temp, temp_value = "[0x00; 8]")]
    _unused5: [u8; 8],
    pub srcs_record: u32, // todo
    pub srcs_count: u32,  // todo
    #[deku(temp, temp_value = "[0xff; 8]")]
    _unused6: [u8; 8],
    pub extra_data_flags: ExtraDataFlags, // todo
    pub ncx_index: u32,
    pub chunk_index: u32,
    pub skel_index: u32,
    pub datp_index: u32,
    pub guide_index: u32,
    #[deku(temp, temp_value = "[0xff; 4]")]
    _unused7: [u8; 4],
    #[deku(temp, temp_value = "[0x00; 4]")]
    _unused8: [u8; 4],
    #[deku(temp, temp_value = "[0xff; 4]")]
    _unused9: [u8; 4],
    #[deku(temp, temp_value = "[0x00; 4]")]
    _unused10: [u8; 4],
    pub exth: Exth,
    #[deku(
        reader = "crate::utils::deku::read_string(deku::reader, *title_length as usize)",
        writer = "crate::utils::deku::write_string(deku::writer, title)"
    )]
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
        #[test]
        fn test_roundtrip(header in any::<MobiHeader>()) {
          let serialized = header.to_bytes().unwrap();

          let ((remaining, _), parsed) = MobiHeader::from_bytes((&serialized, 0)).expect("could not parse");

          assert_eq!(parsed, header);
          assert_eq!(remaining.len(), 0);
        }
    }
}
