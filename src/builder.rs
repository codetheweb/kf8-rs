use cookie_factory::{
    bytes::{be_u16, be_u24, be_u32, be_u8},
    combinator::slice,
    multi::all,
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    constants::{MainLanguage, SubLanguage},
    types::{BookHeader, CompressionType, MobiHeader, MobiHeaderIdent, SectionHeader},
    Codepage,
};

fn write_compression_type<W: Write>(compression_type: &CompressionType) -> impl SerializeFn<W> {
    match compression_type {
        CompressionType::None => cookie_factory::combinator::slice(&[0x00, 0x01]),
        CompressionType::PalmDoc => cookie_factory::combinator::slice(&[0x00, 0x02]),
        CompressionType::HuffCdic => cookie_factory::combinator::slice(&[0x44, 0x48]),
    }
}

fn write_str_with_fixed_length<'a, W: Write + 'a>(
    s: &'a str,
    len: usize,
) -> impl SerializeFn<W> + 'a {
    let s_bytes = s.as_bytes();
    let s_bytes = if s_bytes.len() > len {
        &s_bytes[..len]
    } else {
        s_bytes
    };

    let padding = vec![0x00; len - s_bytes.len()];
    cookie_factory::sequence::pair(slice(s_bytes), slice(padding))
}

fn write_name<'a, W: Write + 'a>(name: &'a str) -> impl SerializeFn<W> + 'a {
    write_str_with_fixed_length(name, 32)
}

fn write_created_at<W: Write>() -> impl SerializeFn<W> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    tuple((
        be_u32(0x00),
        be_u32(now as u32),
        be_u32(now as u32),
        be_u32(0x00),
        be_u32(0x00),
        be_u32(0x00),
        be_u32(0x00),
    ))
}

fn write_ident<W: Write>(ident: &MobiHeaderIdent) -> impl SerializeFn<W> {
    match ident {
        MobiHeaderIdent::BookMobi => cookie_factory::combinator::slice(b"BOOKMOBI"),
        MobiHeaderIdent::TextRead => cookie_factory::combinator::slice(b"TEXTREAD"),
    }
}

fn write_section_header<W: Write>(header: &SectionHeader) -> impl SerializeFn<W> {
    tuple((
        be_u32(header.offset),
        be_u8(header.flags),
        be_u24(header.val),
    ))
}

fn write_num_section_headers<W: Write>(num_sections: u16) -> impl SerializeFn<W> {
    tuple((
        be_u32((num_sections as u32 * 2).wrapping_sub(1)),
        be_u32(0x00),
        be_u16(num_sections),
    ))
}

pub fn write_palmdb_header<'a, W: Write + 'a>(header: &'a MobiHeader) -> impl SerializeFn<W> + 'a {
    tuple((
        write_name(&header.name),
        write_created_at(),
        write_ident(&header.ident),
        // todo: should be called records
        write_num_section_headers(header.num_sections),
        all(header.section_headers.iter().map(write_section_header)),
        be_u16(0x00),
    ))
}

pub fn write_codepage<W: Write>(codepage: &Codepage) -> impl SerializeFn<W> {
    match codepage {
        Codepage::Cp1252 => slice(&[0x00, 0x00, 0x04, 0xe4]),
        Codepage::Utf8 => slice(&[0x00, 0x00, 0xfd, 0xe9]),
    }
}

pub fn write_language<W: Write>(
    language: Option<MainLanguage>,
    sub_language: Option<SubLanguage>,
) -> impl SerializeFn<W> {
    let language = language.map_or(0, |language| u8::from(language) as u32);
    let sub_language = sub_language.map_or(0, |sub_language| u8::from(sub_language) as u32);
    let langcode = (sub_language << 10) | language;
    be_u32(langcode)
}

pub fn write_book_header<'a, W: Write + 'a>(header: &'a BookHeader) -> impl SerializeFn<W> + 'a {
    tuple((
        tuple((
            write_compression_type(&header.compression_type),
            // Unused
            be_u16(0),
            // Text length
            be_u32(0), // todo
            // Number of text records or last record index
            be_u16(header.records),
            // Record size
            be_u16(header.records_size),
            be_u16(header.encryption_type),
            // Unused
            be_u16(0),
            slice("MOBI"),                   // todo
            be_u32(264),                     // todo: length
            be_u32(0),                       // todo: type field
            write_codepage(&Codepage::Utf8), // todo
            be_u32(header.unique_id),
            be_u32(0), // todo: version
        )),
        tuple((
            // Meta orth record
            be_u32(u32::MAX),
            // Meta infl index
            be_u32(u32::MAX),
            // Extra indices
            slice(&[0xff; 32]),
            // First non-text record
            be_u32(0), // todo
            be_u32(0), // todo: title offset
            be_u32(header.title.as_bytes().len() as u32),
            write_language(header.language.clone(), header.sub_language.clone()),
            // 4 bytes of padding?
            slice(&[0x00; 4]),
            be_u32(0), // todo: mobi version
            be_u32(header.first_resource_section_index as u32),
            // Huff/CDIC compression
            slice(&[0x00, 16]),
            // EXTH flags
            be_u32(0), // todo
            // Unknown
            slice(&[0x00; 32]),
        )),
        tuple((
            // Unknown
            be_u32(u32::MAX),
            // DRM offset
            be_u32(u32::MAX),
            // DRM count
            be_u32(0),
            // DRM size
            be_u32(0),
            // DRM flags
            be_u32(0),
            // Unknown
            slice(&[0x00; 8]),
            // FDST record
            be_u32(0), // todo
            // FDST count
            be_u32(0), // todo
            // FLIS record
            be_u32(0), // todo
            // FLIS count
            be_u32(1),
            // Unknown
            slice(&[0x00; 8]),
            // SRCS record
            be_u32(u32::MAX),
            // SRC count
            be_u32(0), // todo
            // Unknown
            slice(&[0xff; 8]),
            // Extra data flags
            be_u32(0), // todo
        )),
        tuple((
            // KF8 indices
            be_u32(header.ncxidx),
            be_u32(0), // todo: chunk index
            be_u32(0), // todo: skel index
            be_u32(0), // todo: datp index
            be_u32(0), // todo: guide index
            // Unknown
            slice(&[0xff; 4]),
            slice(&[0x00; 4]),
            slice(&[0xff; 4]),
            slice(&[0x00; 4]),
            // EXTH
            be_u32(0), // todo
        )),
    ))
}

#[derive(Debug, Default)]
pub struct KF8Builder {
    chapters: Vec<(String, Vec<u8>)>,
}

impl KF8Builder {
    pub fn new() -> KF8Builder {
        KF8Builder::default()
    }

    pub fn add_chapter(&mut self, filename: String, content: Vec<u8>) {
        self.chapters.push((filename, content));
    }

    pub fn write<Writer: Write>(&self, writer: &mut Writer) {
        for (filename, content) in &self.chapters {
            let compressed = palmdoc_compression::compress(content);
        }
    }
}
