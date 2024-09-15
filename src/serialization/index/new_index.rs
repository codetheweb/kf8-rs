use std::iter::once;

use deku::prelude::*;

#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::serialization::{
    tag_map::{TagDefinition, TagMapEntry},
    TagMapDefinition,
};

use super::types::IndexTagMapEntry;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[derive(Debug, PartialEq, Clone)]
struct GeometryBlockInner {
    #[deku(temp, temp_value = "key.len() as u8")]
    key_len: u8,
    #[deku(
        reader = "crate::utils::deku::read_string(deku::reader, *key_len as usize)",
        writer = "crate::utils::deku::write_string(deku::writer, key)"
    )]
    key: String, // "index_num" or "last_idx" in Calibre
    num_records: u16,
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(magic = b"INDX", endian = "big")]
#[derive(Debug, PartialEq)]
struct Header {
    #[deku(temp, temp_value = "192")]
    len: u32,
    #[deku(temp, temp_value = "0")]
    _unused_index_type: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused0: [u8; 4],
    #[deku(temp, temp_value = "2")]
    _unused_header_type: u32,
    pub idxt_offset: u32, // todo: ?
    pub num_of_records: u32,
    #[deku(temp, temp_value = "65001")]
    _unused_encoding: u32,
    #[deku(temp, temp_value = "u32::MAX")]
    _unused_index_language: u32,
    pub total_index_count: u32, // "num_of_entries" in Calibre
    pub ordt_offset: u32,
    pub ligt_offset: u32,
    pub num_of_ordt_ligt_entries: u32,
    pub num_of_cncx_records: u32,
    #[deku(temp, temp_value = "[0; 124]")]
    _unused2: [u8; 124],
    #[deku(temp, temp_value = "len")]
    tagx_offset: u32,
    #[deku(temp, temp_value = "[0; 8]")]
    _unused3: [u8; 8],
    pub tagx: TagMapDefinition,
}

#[deku_derive(DekuWrite)]
#[deku(endian = "big")]
#[derive(Debug, PartialEq)]
struct GeometryBlock {
    pub geometry: Vec<GeometryBlockInner>,
}

// #[deku_derive(DekuRead, DekuWrite)]
// #[derive(Debug, PartialEq)]
// struct IndexBlock {
//   key_len: u32,
//   #[deku(
//       reader = "crate::utils::deku::read_string(deku::reader, *key_len as usize)",
//       writer = "crate::utils::deku::write_string(deku::writer, key)"
//   )]
//   key: String, // "index_num" in Calibre
//   control_bytes: u32, // todo

// }

// TagMapEntry

#[deku_derive(DekuWrite)]
#[deku(magic = b"IDXT", endian = "big")]
struct IdxtBlock {
    key_offsets: Vec<u16>,
}

#[deku_derive(DekuWrite)]
#[deku(magic = b"INDX", endian = "big")]
#[derive(Debug, PartialEq)]
struct IndexRecord {
    len: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused0: [u8; 4],
    #[deku(temp, temp_value = "1")]
    _unused_header_type: u32,
    #[deku(temp, temp_value = "[0; 4]")]
    _unused1: [u8; 4],
    idxt_block_offset: u32,
    num_of_idxt_entries: u32,
    #[deku(temp, temp_value = "[0xff; 8]")]
    _unused2: [u8; 8],
    #[deku(temp, temp_value = "[0; 156]")]
    _unused2: [u8; 156],
    #[deku(skip)]
    tag_definitions: Vec<TagDefinition>,
    #[deku(ctx = "tag_definitions")]
    tag_map_entries: Vec<TagMapEntry>,
    // todo: idxt block
}

#[derive(Debug, PartialEq)]
// #[cfg_attr(test, derive(Arbitrary))]
pub struct TotalIndexEntry {
    tag_definitions: Vec<TagDefinition>,
    entries: Vec<TagMapEntry>,
}

impl TotalIndexEntry {
    pub fn new(tag_definitions: Vec<TagDefinition>, entries: Vec<TagMapEntry>) -> Self {
        Self {
            tag_definitions,
            entries,
        }
    }

    // todo: should record be a type alias?
    pub fn into_records(self) -> Vec<Vec<u8>> {
        let mut records = Vec::new();

        let mut geometry = Vec::new();
        // let mut key_offsets: Vec<u16> = Vec::new();
        for entry in self.entries.clone() {
            let geometry_block = GeometryBlockInner {
                key: entry.text.clone(),
                num_records: 1,
            };
            geometry.push(geometry_block);

            // match key_offsets.last() {
            //     Some(offset) => key_offsets.push(offset + entry.text.len() as u16),
            //     None => key_offsets.push(0),
            // }
        }

        // todo: implement splitting for blocks over max record size
        let mut header = Header {
            idxt_offset: 0, // replaced later
            num_of_records: 1,
            total_index_count: self.entries.len() as u32,
            ordt_offset: 0,
            ligt_offset: 0,
            num_of_ordt_ligt_entries: 0,
            num_of_cncx_records: 0,
            tagx: TagMapDefinition {
                tag_definitions: self.tag_definitions.clone(),
            },
        };

        let geometry_block = GeometryBlock {
            geometry: geometry.clone(),
        };

        let header_bytes = header.to_bytes().unwrap();
        let geometry_bytes = geometry_block.to_bytes().unwrap();
        header.idxt_offset = (header_bytes.len() + geometry_bytes.len()) as u32;
        let header_bytes = header.to_bytes().unwrap();

        println!("header length: {}", header_bytes.len());

        let idxt_block = IdxtBlock {
            key_offsets: once(0)
                .chain(geometry.iter().map(|x| x.key.len() as u16))
                .map(|x| header_bytes.len() as u16 + x)
                // skip last
                .enumerate()
                .filter_map(|(i, x)| if i == geometry.len() { None } else { Some(x) })
                .collect(),
        };

        let header_bytes = [header_bytes, geometry_bytes, idxt_block.to_bytes().unwrap()].concat();
        records.push(header_bytes);

        // Create index record
        let index_record = IndexRecord {
            len: 192,
            idxt_block_offset: 0,
            num_of_idxt_entries: 0,
            tag_definitions: self.tag_definitions,
            tag_map_entries: self.entries,
        };
        let index_record_bytes = index_record.to_bytes().unwrap();
        let index_record_bytes = [index_record_bytes, "IDXT".as_bytes().to_vec()].concat();
        records.push(index_record_bytes);

        // todo: cncx records

        records
    }
}
