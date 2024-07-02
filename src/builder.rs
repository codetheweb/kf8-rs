use cookie_factory::{
    bytes::{be_u16, be_u24, be_u32, be_u8},
    combinator::slice,
    multi::all,
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{CompressionType, MobiHeader, MobiHeaderIdent, SectionHeader};

fn write_compression_type<W: Write>(compression_type: CompressionType) -> impl SerializeFn<W> {
    match compression_type {
        CompressionType::None => cookie_factory::combinator::slice(&[0x00, 0x01]),
        CompressionType::PalmDoc => cookie_factory::combinator::slice(&[0x00, 0x02]),
        CompressionType::HuffCdic => cookie_factory::combinator::slice(&[0x44, 0x48]),
    }
}

fn write_name<'a, W: Write + 'a>(name: &'a str) -> impl SerializeFn<W> + 'a {
    let name_bytes = name.as_bytes();
    let name_bytes = if name_bytes.len() > 31 {
        &name_bytes[..31]
    } else {
        name_bytes
    };

    let padding = vec![0x00; 32 - name_bytes.len()];
    cookie_factory::sequence::pair(slice(name_bytes), slice(padding))
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

fn write_ident<W: Write>(ident: MobiHeaderIdent) -> impl SerializeFn<W> {
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

pub fn write_mobi_header<'a, W: Write + 'a>(header: &'a MobiHeader) -> impl SerializeFn<W> + 'a {
    tuple((
        write_name(&header.name),
        write_created_at(),
        write_ident(header.ident.clone()),
        write_num_section_headers(header.num_sections),
        all(header.section_headers.iter().map(write_section_header)),
        be_u16(0x00),
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
