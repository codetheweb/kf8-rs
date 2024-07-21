use deku::prelude::*;
use nom::{
    bytes::complete::take,
    error::Error,
    multi::count,
    number::complete::{be_u16, be_u8},
    IResult,
};
use serialization::{FDSTTable, IndxHeader, MobiHeader, PalmDoc, TagTable};
use std::{collections::HashMap, str::FromStr};

use crate::{constants::MetadataIdValue, tag_map::parse_tag_map};

pub mod constants;
mod serialization;
mod tag_map;
mod utils;

#[derive(Debug, PartialEq)]
pub struct MobiBookFragment {
    pub index: usize,
    pub content: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct MobiBookPart {
    pub filename: String,
    pub skeleton_head: Vec<u8>,
    pub fragments: Vec<MobiBookFragment>,
    pub skeleton_tail: Vec<u8>,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl MobiBookPart {
    pub fn get_content(&self) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(&self.skeleton_head);
        for fragment in &self.fragments {
            content.extend_from_slice(&fragment.content);
        }
        content.extend_from_slice(&self.skeleton_tail);
        content
    }
}

#[derive(Debug, PartialEq)]
pub enum ImageResourceKind {
    Cover,
    Thumbnail,
    Other,
}

#[derive(Debug, PartialEq)]
pub enum ResourceKind {
    Image(ImageResourceKind),
    Font,
    Stylesheet,
}

#[derive(Debug)]
pub struct Resource {
    pub kind: ResourceKind,
    pub data: Vec<u8>,
    pub file_type: infer::Type,
    pub flow_index: Option<usize>,
}

#[derive(Debug)]
pub struct MobiBook {
    palmdoc: PalmDoc,
    pub book_header: MobiHeader,
    pub fragment_table: Vec<FragmentTableEntry>,
    content: String,
    pub parts: Vec<MobiBookPart>,
    pub resources: Vec<Resource>,
}

pub fn parse_book(input: &[u8]) -> IResult<&[u8], MobiBook> {
    let original_input = input;
    let (_, palmdoc) = PalmDoc::from_bytes((&input, 0)).expect("could not parse header");

    let (input, _) = take(2usize)(input)?; // Skip 2 bytes

    // todo: use first section offset instead of manually skipping bytes above?
    let (_, book_header) =
        crate::serialization::MobiHeader::from_bytes((&palmdoc.records.first().unwrap(), 0))
            .expect("could not parse header");

    // todo: assert that header is k8?

    let mut raw_ml = Vec::new();
    for (i, section_header) in palmdoc.records.iter().enumerate().skip(1) {
        if i > book_header.last_text_record as usize {
            break;
        }

        let section_data = palmdoc.records[i].as_slice();
        let section_data = &section_data
            [..section_data.len() - book_header.sizeof_trailing_section_entries(section_data)];

        let decompressed = palmdoc_compression::calibre::decompress(section_data);

        raw_ml.extend_from_slice(&decompressed);
    }

    // Parse flow boundaries
    let fdst_section_data = palmdoc.records[book_header.fdst_record as usize].as_slice();

    let (_, fdst_table) = FDSTTable::from_bytes((fdst_section_data, 0)).unwrap();

    let mut flows = Vec::new();

    for entry in fdst_table.entries {
        let flow = &raw_ml[(entry.start as usize)..(entry.end as usize)];
        flows.push(flow);
    }

    let text = *flows.first().unwrap();

    let (_, skeleton_table) =
        parse_index_data(original_input, &palmdoc, book_header.skel_index as usize).unwrap();

    let (_, fragment_table) =
        parse_index_data(original_input, &palmdoc, book_header.chunk_index as usize).unwrap();

    let fragment_table = index_table_to_fragment_table(&fragment_table);

    let mut parts = vec![];

    let mut fragment_i = 0;
    for (i, skeleton_entry) in index_table_to_skeleton_table(&skeleton_table)
        .iter()
        .enumerate()
    {
        let mut base_ptr = skeleton_entry.start_offset + skeleton_entry.len;

        let mut fragments: Vec<MobiBookFragment> = vec![];

        let first_fragment = fragment_table.get(fragment_i).unwrap();
        let split_skeleton_at = first_fragment.insert_position as usize;

        // todo: zip?
        let mut filename: String = "".to_string();
        for i in 0..skeleton_entry.fragment_table_record_count {
            let fragment_entry = fragment_table.get(fragment_i).unwrap();

            if i == 0 {
                filename = format!("part{}.xhtml", fragment_entry.file_number);
            }

            let fragment_text = &text[base_ptr..base_ptr + fragment_entry.len as usize];

            fragments.push(MobiBookFragment {
                index: fragment_i,
                content: fragment_text.to_vec(),
            });

            base_ptr += fragment_entry.len as usize;
            fragment_i += 1;
        }

        let skeleton_head = &text[skeleton_entry.start_offset..split_skeleton_at];
        let skeleton_tail =
            &text[split_skeleton_at..skeleton_entry.start_offset + skeleton_entry.len];

        parts.push(MobiBookPart {
            filename,
            skeleton_head: skeleton_head.to_vec(),
            fragments,
            skeleton_tail: skeleton_tail.to_vec(),
            start_offset: skeleton_entry.start_offset,
            end_offset: base_ptr,
        });
    }

    // Resources
    let mut resources: Vec<Resource> = vec![];

    // todo: handle SVGs/images, CDATA?
    let stylesheets = flows.iter().skip(1);

    let mut info = infer::Infer::new();
    info.add("text/css", "css", |_| true);

    for (i, stylesheet) in stylesheets.enumerate() {
        resources.push(Resource {
            kind: ResourceKind::Stylesheet,
            data: stylesheet.to_vec(),
            file_type: info.get(stylesheet).unwrap(),
            flow_index: Some(i + 1),
        });
    }

    let cover_offset = book_header.first_resource_record as usize
        + *book_header
            .exth
            .metadata_value
            .get(&MetadataIdValue::CoverOffset)
            .unwrap()
            .first()
            .unwrap() as usize;

    let thumbnail_offset = book_header.first_resource_record as usize
        + *book_header
            .exth
            .metadata_value
            .get(&MetadataIdValue::ThumbOffset)
            .unwrap()
            .first()
            .unwrap() as usize;

    for section_i in book_header.first_resource_record as usize..palmdoc.records.len() {
        let data = palmdoc.records[section_i].as_slice();
        let (input, resource_type) = take::<usize, &[u8], Error<&[u8]>>(4usize)(data).unwrap();

        match resource_type {
            b"FLIS" | b"FCIS" | b"FDST" | b"DATP" => {
                // todo?
            }
            b"SRCS" => {
                // todo
            }
            b"PAGE" => {
                // todo
            }
            b"CMET" => {
                // todo
            }
            b"FONT" => {
                // todo
            }
            b"CRES" => {
                // todo
            }
            b"CONT" => {
                // todo
            }
            b"kind" => {
                // todo
            }
            [0xa0, 0xa0, 0xa0, 0xa0] => {
                // todo
                println!("byte pattern, empty image?")
            }
            b"RESC" => {
                // todo
            }
            // EOF
            [0xe9, 0x8e, 0x0d, 0x0a] => {
                // todo
            }
            b"BOUN" => {
                // todo
            }
            _ => {
                // Should be an image
                let file_type = infer::get(data);

                if section_i == cover_offset {
                    resources.push(Resource {
                        kind: ResourceKind::Image(ImageResourceKind::Cover),
                        data: data.to_vec(),
                        file_type: file_type.unwrap(),
                        flow_index: None,
                    })
                } else if section_i == thumbnail_offset {
                    resources.push(Resource {
                        kind: ResourceKind::Image(ImageResourceKind::Thumbnail),
                        data: data.to_vec(),
                        file_type: file_type.unwrap(),
                        flow_index: None,
                    })
                } else {
                    resources.push(Resource {
                        kind: ResourceKind::Image(ImageResourceKind::Other),
                        data: data.to_vec(),
                        file_type: file_type.unwrap(),
                        flow_index: None,
                    })
                }
            }
        }
    }

    Ok((
        input,
        MobiBook {
            palmdoc: palmdoc.clone(),
            book_header,
            fragment_table,
            // todo: this should not be lossy
            content: String::from_utf8_lossy(&raw_ml).to_string(),
            parts,
            resources,
        },
    ))
}

#[derive(Debug)]
struct IndexTableEntry {
    file_number: usize,
    label: String,
    tag_map: HashMap<u8, Vec<u32>>,
}

fn parse_index_data<'a>(
    original_input: &'a [u8],
    palmdoc: &'a PalmDoc,
    section_i: usize,
) -> IResult<&'a [u8], Vec<IndexTableEntry>> {
    // Parse INDX header
    let indx_section_data = palmdoc.records[section_i].as_slice();
    let (_, indx_header) = parse_indx_header(indx_section_data).unwrap();

    let (_, tag_section) =
        TagTable::from_bytes((&indx_section_data[indx_header.len as usize..], 0)).unwrap();

    let mut index_table = vec![];

    for i in (section_i + 1)..(section_i + 1 + indx_header.num_entries as usize) {
        let data = palmdoc.records[i].as_slice();
        let (_, header) = parse_indx_header(data).unwrap();

        let (_, indx_offsets) =
            count(be_u16, header.num_entries as usize)(&data[header.block_offset as usize + 4..])?;

        for (i, beginning_offset) in indx_offsets.iter().enumerate() {
            let (remaining, segment) =
                parse_indx_text_segment(&data[*beginning_offset as usize..])?;

            let (_, tag_map) = parse_tag_map(&tag_section.tags, remaining).unwrap();

            index_table.push(IndexTableEntry {
                file_number: i,
                label: segment,
                tag_map,
            });
        }
    }

    Ok((&[], index_table))
}

#[derive(Debug)]
struct SkeletonTableEntry {
    file_number: usize,
    label: String,
    fragment_table_record_count: usize,
    start_offset: usize,
    len: usize,
}

fn index_table_to_skeleton_table(table_entries: &[IndexTableEntry]) -> Vec<SkeletonTableEntry> {
    table_entries
        .iter()
        .map(|entry| SkeletonTableEntry {
            file_number: entry.file_number,
            label: entry.label.clone(),
            fragment_table_record_count: entry.tag_map.get(&1).unwrap()[0] as usize,
            start_offset: entry.tag_map.get(&6).unwrap()[0] as usize,
            len: entry.tag_map.get(&6).unwrap()[1] as usize,
        })
        .collect()
}

#[derive(Debug)]
pub struct FragmentTableEntry {
    pub insert_position: u32,
    id_text: String,
    file_number: u32,
    seq_number: u32,
    start_pos: u32,
    len: u32,
}

fn index_table_to_fragment_table(table_entries: &[IndexTableEntry]) -> Vec<FragmentTableEntry> {
    table_entries
        .iter()
        .map(|entry| FragmentTableEntry {
            insert_position: u32::from_str(&entry.label).unwrap(),
            // todo
            id_text: "".to_string(),
            file_number: entry.tag_map.get(&3).unwrap()[0],
            seq_number: entry.tag_map.get(&4).unwrap()[0],
            start_pos: entry.tag_map.get(&6).unwrap()[0],
            len: entry.tag_map.get(&6).unwrap()[1],
        })
        .collect()
}

fn parse_indx_text_segment(input: &[u8]) -> IResult<&[u8], String> {
    let (input, len) = be_u8(input)?;
    let (input, segment) = take(len as usize)(input)?;

    // todo: remove unwrap
    Ok((input, String::from_utf8(segment.to_vec()).unwrap()))
}

fn parse_indx_header(input: &[u8]) -> IResult<&[u8], IndxHeader> {
    let ((input, _), header) = IndxHeader::from_bytes((input, 0)).expect("could not parse header");

    Ok((input, header))
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn extract_raw_html() {
        // todo
        env_logger::init();
        let mut reader = std::fs::File::open("resources/war_and_peace.azw3").unwrap();
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();

        let (_, book) = parse_book(&data).expect("could not parse book");

        let mut expected_html_reader =
            std::fs::File::open("resources/war_and_peace.rawml").unwrap();
        let mut expected_html = String::new();
        expected_html_reader
            .read_to_string(&mut expected_html)
            .unwrap();

        assert_eq!(book.content, expected_html);
    }
}
