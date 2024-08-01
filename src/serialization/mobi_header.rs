use std::io::{Read, Write};

use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::constants::{MainLanguage, SubLanguage};

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

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ExthFlags {
    pub has_exth: bool,
    pub has_fonts: bool,
    pub is_periodical: bool,
}

impl<'a, Ctx> DekuReader<'a, Ctx> for ExthFlags {
    fn from_reader_with_ctx<R: Read>(reader: &mut Reader<R>, ctx: Ctx) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let flags = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Big)?;
        let has_exth = flags & 0b1010000 != 0;
        let has_fonts = flags & 0b1000000000000 != 0;
        let is_periodical = flags & 0b1000 != 0;

        Ok(ExthFlags {
            has_exth,
            has_fonts,
            is_periodical,
        })
    }
}

impl<Ctx> DekuWriter<Ctx> for ExthFlags {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _ctx: Ctx) -> Result<(), DekuError> {
        let mut flags = 0;
        if self.has_exth {
            flags |= 0b1010000;
        }
        if self.has_fonts {
            flags |= 0b1000000000000;
        }
        if self.is_periodical {
            flags |= 0b1000;
        }

        flags.to_writer(writer, deku::ctx::Endian::Big)
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Codepage {
    #[deku(id = "0x000004e4")]
    Cp1252,
    #[deku(id = "0x0000fde9")]
    Utf8,
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

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
#[deku(id_type = "u32", ctx = "endian: deku::ctx::Endian", endian = "endian")]
pub enum BookType {
    #[deku(id = 0x2)]
    Book,
    #[deku(id = 0x101)]
    NewsHierarchal,
    #[deku(id = 0x102)]
    NewsFlat,
    #[deku(id = 0x103)]
    NewsMagazine,
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(
    test,
    proptest(filter = "|header| header.exth_flags.has_exth == header.exth.is_some()")
)]
pub struct MobiHeader {
    pub compression_type: CompressionType,
    #[deku(temp, temp_value = "[0x00; 2]")]
    _unused0: [u8; 2],
    pub text_length: u32,
    pub last_text_record: u16,
    pub text_record_size: u16,
    #[deku(temp, temp_value = "0")]
    encryption_type: u16,
    #[deku(temp, temp_value = "[0x00; 2]")]
    _unused1: [u8; 2],
    #[deku(temp, temp_value = "*b\"MOBI\"")]
    ident: [u8; 4], // todo
    #[deku(temp, temp_value = "264")]
    header_length: u32,
    pub book_type: BookType,
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
    pub title_offset: u32, // todo: derive with deku?
    #[deku(temp, temp_value = "title.len() as u32")]
    pub title_length: u32,
    pub language_code: LanguageCode, // language_code
    #[deku(temp, temp_value = "0")]
    in_lang: u32,
    #[deku(temp, temp_value = "0")]
    out_lang: u32,
    #[deku(temp, temp_value = "*file_version")]
    min_version: u32,
    pub first_resource_record: u32,
    #[deku(temp, temp_value = "0")]
    huff_first_record: u32,
    #[deku(temp, temp_value = "0")]
    huff_count: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    huff_table_offset: [u8; 4],
    #[deku(temp, temp_value = "[0; 4]")]
    huff_table_length: [u8; 4],
    pub exth_flags: ExthFlags, // todo
    #[deku(temp, temp_value = "[0; 32]")]
    _unused2: [u8; 32],
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
    // todo: add a deku assert to relate to flags
    #[deku(cond = "exth_flags.has_exth")]
    pub exth: Option<Exth>,
    #[deku(
        reader = "crate::utils::deku::read_string(deku::reader, *title_length as usize)",
        writer = "crate::utils::deku::write_string(deku::writer, title)"
    )]
    pub title: String,
}

impl MobiHeader {
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

        let mut encoded_flags = self.extra_data_flags.encode() >> 1;

        while encoded_flags > 0 {
            if encoded_flags & 1 > 0 {
                num += sizeof_trailing_section_entry(section_data, size - num);
            }

            encoded_flags >>= 1;
        }

        if self
            .extra_data_flags
            .extra_multibyte_bytes_after_text_records
        {
            let offset = size - num - 1;
            num += (section_data[offset] as usize & 0x3) + 1;
        }

        num
    }

    pub fn get_bcp47_language_tag(&self) -> Option<&'static str> {
        return self.language_code.sub.as_ref().map_or(
            self.language_code.main.as_ref().map(|l| l.to_bcp47()),
            |l| Some(l.to_bcp47()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
        #[test]
        fn test_mobi_header_roundtrip(header in any::<MobiHeader>()) {
          let serialized = header.to_bytes().unwrap();

          let ((remaining, _), parsed) = MobiHeader::from_bytes((&serialized, 0)).expect("could not parse");

          assert_eq!(parsed, header);
          assert_eq!(remaining.len(), 0);
        }
    }
}
