use std::io::SeekFrom;

use binrw::{prelude::*, NullString, PosValue};
use deku::{DekuReader, DekuWriter};
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::constants::{MainLanguage, SubLanguage};

use super::exth::Exth;

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
#[binrw]
#[brw(big)]
struct SerializedExth {
    #[br(parse_with = parse_exth)]
    #[bw(write_with = write_exth)]
    exth: Exth,
}

#[binrw::parser(reader)]
fn parse_exth() -> BinResult<Exth> {
    let mut deku_reader = deku::reader::Reader::new(reader);
    let parsed = Exth::from_reader_with_ctx(&mut deku_reader, ()).unwrap();
    Ok(parsed)
}

#[binrw::writer(writer)]
fn write_exth(exth: &Exth) -> BinResult<()> {
    let mut deku_writer = deku::writer::Writer::new(writer);
    exth.to_writer(&mut deku_writer, ()).unwrap();
    Ok(())
}

#[binrw::parser(reader, endian)]
fn parse_language_code() -> BinResult<LanguageCode> {
    let langcode = u32::read_options(reader, endian, ())?;

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

#[binrw::writer(writer, endian)]
fn write_language_code(langcode: &LanguageCode) -> BinResult<()> {
    let language = langcode
        .main
        .clone()
        .map_or(0, |language| u8::from(language) as u32);
    let sub_language = langcode
        .sub
        .clone()
        .map_or(0, |sub_language| u8::from(sub_language) as u32);
    let langcode = (sub_language << 10) | language;

    langcode.write_options(writer, endian, ())
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct LanguageCode {
    pub main: Option<MainLanguage>,
    pub sub: Option<SubLanguage>,
}

#[derive(Debug, PartialEq)]
#[binrw]
#[brw(big)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SerializedLanguageCode {
    #[br(parse_with = parse_language_code)]
    #[bw(write_with = write_language_code)]
    language_code: LanguageCode,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ExtraDataFlags {
    pub extra_multibyte_bytes_after_text_records: bool,
    pub has_tbs: bool,
    pub uncrossable_breaks: bool,
}

// todo: should this be pub?
#[binrw]
#[brw(big)]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SerializedExtraDataFlags {
    #[br(parse_with = parse_extra_data_flags)]
    #[bw(write_with = write_extra_data_flags)]
    pub flags: ExtraDataFlags,
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

#[binrw::parser(reader, endian)]
fn parse_extra_data_flags() -> BinResult<ExtraDataFlags> {
    let flags = u32::read_options(reader, endian, ())?;

    Ok(ExtraDataFlags {
        extra_multibyte_bytes_after_text_records: (flags & 0b1) != 0,
        has_tbs: (flags & 0b10) != 0,
        uncrossable_breaks: (flags & 0b100) != 0,
    })
}

#[binrw::writer(writer, endian)]
fn write_extra_data_flags(flags: &ExtraDataFlags) -> BinResult<()> {
    flags.encode().write_options(writer, endian, ())
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ExthFlags {
    pub has_exth: bool,
    pub has_fonts: bool,
    pub is_periodical: bool,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
#[binrw]
#[brw(big)]
pub struct SerializedExthFlags {
    #[br(parse_with = parse_exth_flags)]
    #[bw(write_with = write_exth_flags)]
    pub flags: ExthFlags,
}

#[binrw::parser(reader, endian)]
fn parse_exth_flags() -> BinResult<ExthFlags> {
    let flags = u32::read_options(reader, endian, ())?;
    let has_exth = flags & 0b1010000 != 0;
    let has_fonts = flags & 0b1000000000000 != 0;
    let is_periodical = flags & 0b1000 != 0;

    Ok(ExthFlags {
        has_exth,
        has_fonts,
        is_periodical,
    })
}

#[binrw::writer(writer, endian)]
fn write_exth_flags(flags: &ExthFlags) -> BinResult<()> {
    let mut encoded = 0;
    if flags.has_exth {
        encoded |= 0b1010000;
    }
    if flags.has_fonts {
        encoded |= 0b1000000000000;
    }
    if flags.is_periodical {
        encoded |= 0b1000;
    }

    encoded.write_options(writer, endian, ())
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
#[binrw]
#[brw(big, repr(u32))]
pub enum Codepage {
    Cp1252 = 0x000004e4,
    Utf8 = 0x0000fde9,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
#[binrw]
#[brw(big, repr(u16))]
pub enum CompressionType {
    // todo: should None be an option?
    None = 0x0001,
    PalmDoc = 0x0002,
    HuffCdic = 0x4448,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
#[binrw]
#[brw(big, repr(u32))]
pub enum BookType {
    Book = 0x2,
    NewsHierarchal = 0x101,
    NewsFlat = 0x102,
    NewsMagazine = 0x103,
}

#[cfg(test)]
fn any_null_string() -> impl proptest::prelude::Strategy<Value = NullString> {
    use proptest::prelude::Strategy;

    // todo: 64 is arbitrary, allow all UTF-8
    "[a-zA-Z0-9]{0, 64}".prop_map(|v| NullString(v.into()))
}

#[binrw::writer(writer, endian)]
fn write_string_and_offset(s: &NullString) -> BinResult<()> {
    let current_position = writer.stream_position()?;
    println!("current_position: {}", current_position);
    writer.seek(SeekFrom::Start(0x54))?;

    let offset: u32 = current_position as u32;
    offset.write_options(writer, endian, ())?;

    writer.seek(SeekFrom::Start(current_position))?;
    s.write(writer)?;
    Ok(())
}

#[binrw]
#[brw(big)]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(
    test,
    proptest(filter = "|header| header.exth_flags.has_exth == header.exth.is_some()")
)]
pub struct MobiHeader {
    pub compression_type: CompressionType,
    #[br(temp)]
    #[bw(calc = [0x00; 2])]
    _unused0: [u8; 2],
    pub text_length: u32,
    pub num_of_text_records: u16,
    pub text_record_size: u16,
    #[br(temp)]
    #[bw(calc = 0)]
    encryption_type: u16,
    #[br(temp)]
    #[bw(calc = [0x00; 2])]
    _unused1: [u8; 2],
    #[br(temp)]
    #[bw(calc = *b"MOBI")]
    ident: [u8; 4], // todo
    #[br(temp)]
    #[bw(calc = 264)]
    header_length: u32,
    pub book_type: BookType,
    pub text_encoding: Codepage,
    pub uid: u32,          // todo
    pub file_version: u32, // todo 36
    #[br(temp)]
    #[bw(calc = u32::MAX)]
    pub meta_orth_record: u32,
    #[br(temp)]
    #[bw(calc = u32::MAX)]
    pub meta_infl_index: u32, // 44
    #[br(temp)]
    #[bw(calc = [u32::MAX; 8])]
    pub extra_indices: [u32; 8], // 72
    pub first_non_text_record: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    title_offset: u32, // 0x10e
    #[br(temp)]
    #[bw(calc = title.len() as u32)]
    pub title_length: u32,
    #[br(map = |v: SerializedLanguageCode| v.language_code)]
    #[bw(map = |v: &LanguageCode| SerializedLanguageCode { language_code: v.clone() })]
    pub language_code: LanguageCode,
    #[br(temp)]
    #[bw(calc = 0)]
    in_lang: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    out_lang: u32,
    #[br(temp)]
    #[bw(calc = *file_version)]
    min_version: u32,
    pub first_resource_record: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    huff_first_record: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    huff_count: u32,
    #[br(temp)]
    #[bw(calc = [0; 4])]
    huff_table_offset: [u8; 4],
    #[br(temp)]
    #[bw(calc = [0; 4])]
    huff_table_length: [u8; 4],
    #[br(map = |v: SerializedExthFlags| v.flags)]
    #[bw(map = |v: &ExthFlags| SerializedExthFlags { flags: v.clone() })]
    pub exth_flags: ExthFlags, // todo
    #[br(temp)]
    #[bw(calc = [0; 32])]
    _unused2: [u8; 32],
    #[br(temp)]
    #[bw(calc = u32::MAX)]
    _unused3: u32,
    #[br(temp)]
    #[bw(calc = u32::MAX)]
    drm_offset: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    drm_count: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    drm_size: u32,
    #[br(temp)]
    #[bw(calc = 0)]
    drm_flags: u32,
    #[br(temp)]
    #[bw(calc = [0x00; 8])]
    _unused4: [u8; 8],
    pub fdst_record: u32, // todo
    pub fdst_count: u32,  // todo
    pub fcis_record: u32, // todo
    // #[deku(assert_eq = "1")]
    pub fcis_count: u32,  // todo
    pub flis_record: u32, // todo
    // #[deku(assert_eq = "1")]
    pub flis_count: u32, // todo
    #[br(temp)]
    #[bw(calc = [0x00; 8])]
    _unused5: [u8; 8],
    pub srcs_record: u32, // todo
    pub srcs_count: u32,  // todo
    #[br(temp)]
    #[bw(calc = [0xff; 8])]
    _unused6: [u8; 8],
    #[br(map = |v: SerializedExtraDataFlags| v.flags)]
    #[bw(map = |v: &ExtraDataFlags| SerializedExtraDataFlags { flags: v.clone() })]
    pub extra_data_flags: ExtraDataFlags, // todo
    pub ncx_index: u32,
    pub chunk_index: u32,
    pub skel_index: u32,
    pub datp_index: u32,
    pub guide_index: u32,
    #[br(temp)]
    #[bw(calc = [0xff; 4])]
    _unused7: [u8; 4],
    #[br(temp)]
    #[bw(calc = [0x00; 4])]
    _unused8: [u8; 4],
    #[br(temp)]
    #[bw(calc = [0xff; 4])]
    _unused9: [u8; 4],
    #[br(temp)]
    #[bw(calc = [0x00; 4])]
    _unused10: [u8; 4],
    // todo: add a deku assert to relate to flags
    #[br(if(exth_flags.has_exth), map = |v: Option<SerializedExth>| v.map(|v| v.exth))]
    #[bw(if(exth_flags.flags.has_exth), map = |v: &Option<Exth>| v.clone().map(|v| SerializedExth { exth: v }))]
    pub exth: Option<Exth>,
    #[cfg_attr(test, proptest(strategy = "any_null_string()"))]
    #[bw(write_with = write_string_and_offset)]
    pub title: NullString,
    #[br(temp, count = 8192)] // todo?
    #[bw(calc = vec![0x00; 8192])]
    padding: Vec<u8>,
}

impl Default for MobiHeader {
    fn default() -> Self {
        MobiHeader {
            compression_type: CompressionType::None,
            text_length: 0,
            num_of_text_records: 0,
            text_record_size: 0,
            book_type: BookType::Book,
            text_encoding: Codepage::Cp1252,
            uid: 0,
            file_version: 0,
            first_non_text_record: 0,
            language_code: LanguageCode {
                main: None,
                sub: None,
            },
            first_resource_record: 0,
            exth_flags: ExthFlags {
                has_exth: false,
                has_fonts: false,
                is_periodical: false,
            },
            fdst_record: 0,
            fdst_count: 0,
            fcis_record: 0,
            fcis_count: 0,
            flis_record: 0,
            flis_count: 0,
            srcs_record: 0,
            srcs_count: 0,
            extra_data_flags: ExtraDataFlags {
                extra_multibyte_bytes_after_text_records: false,
                has_tbs: false,
                uncrossable_breaks: false,
            },
            ncx_index: 0,
            chunk_index: 0,
            skel_index: 0,
            datp_index: 0,
            guide_index: 0,
            exth: None,
            title: String::new().into(),
        }
    }
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
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
        #[test]
        fn test_mobi_header_roundtrip(header in any::<MobiHeader>()) {
            let mut serialized = Cursor::new(Vec::new());
            header.write(&mut serialized).expect("could not serialize");
            serialized.set_position(0);

            let parsed = MobiHeader::read(&mut serialized).expect("could not parse");
            assert_eq!(parsed, header);
        }
    }
}
