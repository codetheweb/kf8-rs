use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek, Write},
};

use deku::prelude::*;
use nom::{bytes::complete::take, error::ErrorKind, number::complete::be_u8, IResult};
#[cfg(test)]
use proptest_derive::Arbitrary;
use thiserror::Error;

use crate::tag_map::parse_tag_map;

use super::{IndxHeader, TagDefinition};

#[derive(Debug, PartialEq)]
pub struct IndexRecord {
    header: IndxHeader,
    pub rows: Vec<TagTableRow>,
}

impl<'a> DekuReader<'a, &Vec<TagDefinition>> for IndexRecord {
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        tag_definitions: &Vec<TagDefinition>,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let header = IndxHeader::from_reader_with_ctx(reader, ())?;

        // todo: this is dumb
        let header_length = header.to_bytes().unwrap().len();
        // todo: where does +4 come from?
        let row_data_length = header.block_offset as usize - header_length + 4;

        let mut rows_buf = Vec::with_capacity(row_data_length);
        for _ in 0..row_data_length {
            let mut buf = [0u8];
            reader.read_bytes_const(&mut buf)?;
            rows_buf.push(buf[0]);
        }

        let mut row_offsets = Vec::new();
        for _ in 0..header.num_entries {
            let mut buf = [0; 2];
            reader.read_bytes(2, &mut buf)?;
            let offset = u16::from_be_bytes(buf);
            row_offsets.push(offset as usize);
        }

        let mut rows = Vec::new();

        for (beginning_offset, end_offset) in row_offsets
            .iter()
            .zip(
                row_offsets
                    .iter()
                    .skip(1)
                    .chain(std::iter::once(&(header.block_offset as usize))),
            )
            .map(|(start, end)| (start - header_length, end - header_length))
        {
            // todo: use single reader/cursor
            let mut cursor = Cursor::new(&rows_buf[beginning_offset..end_offset]);
            let mut reader = Reader::new(&mut cursor);

            let row = TagTableRow::from_reader_with_ctx(
                &mut reader,
                ((end_offset - beginning_offset) as usize, tag_definitions),
            )
            .unwrap();

            rows.push(row);
        }

        Ok(IndexRecord { header, rows })
    }
}

#[derive(Debug, PartialEq)]
pub struct TagTableRow {
    pub text: String,
    pub tag_map: HashMap<u8, Vec<u32>>,
}

impl Default for TagTableRow {
    fn default() -> Self {
        TagTableRow {
            text: "".to_string(),
            tag_map: HashMap::new(),
        }
    }
}

fn read_tag_table_row<'a>(
    input: &'a [u8],
    table_definition: &Vec<TagDefinition>,
) -> IResult<&'a [u8], TagTableRow> {
    let (input, text_length) = be_u8(input)?;
    let (input, text) = take(text_length as usize)(input)?;
    // remap error
    let text = String::from_utf8(text.to_vec())
        .map_err(|_| nom::Err::Error(nom::error::make_error(input, ErrorKind::IsNot)))?;

    let (_, tag_map) = parse_tag_map(table_definition, input)?;

    Ok((input, TagTableRow { text, tag_map }))
}

// ctx is byte length of the row
impl<'a> DekuReader<'a, (usize, &Vec<TagDefinition>)> for TagTableRow {
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        ctx: (usize, &Vec<TagDefinition>),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let (length, table_definition) = ctx;
        let mut buf = vec![0; length];
        reader.read_bytes(length, &mut buf)?;
        let (_, row) = read_tag_table_row(&buf, table_definition).unwrap();
        Ok(row)
    }
}

impl DekuWriter<()> for TagTableRow {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _: ()) -> Result<(), DekuError> {
        let text = self.text.clone();

        writer.write_bytes(&[text.len() as u8])?;
        writer.write_bytes(text.as_bytes())?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TagTableRowParseError {
    #[error("Tag {0} not found in table")]
    TagNotFound(String),
    #[error("Error parsing tag value")]
    ParseError,
}

trait Index<'a>: TryFrom<&'a TagTableRow, Error = TagTableRowParseError> + Into<TagTableRow> {
    fn get_tag_definitions() -> Vec<TagDefinition>;
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ChunkIndex {
    // todo: rename?
    pub insert_position: u32,

    pub cncx_offset: u32,
    pub file_number: u32,
    pub sequence_number: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> TryFrom<&'a TagTableRow> for ChunkIndex {
    type Error = TagTableRowParseError;

    fn try_from(row: &TagTableRow) -> Result<Self, Self::Error> {
        let insert_position: u32 = row
            .text
            .parse()
            .map_err(|_| TagTableRowParseError::ParseError)?;

        let cncx_offset = row
            .tag_map
            .get(&2)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("cncx_offset".to_string()))?[0];
        let file_number = row
            .tag_map
            .get(&3)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("file_number".to_string()))?[0];
        let sequence_number = row
            .tag_map
            .get(&4)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("sequence_number".to_string()))?[0];
        let geometry_pair = row
            .tag_map
            .get(&6)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(ChunkIndex {
            insert_position,
            cncx_offset,
            file_number,
            sequence_number,
            start_offset,
            length,
        })
    }
}

impl Into<TagTableRow> for ChunkIndex {
    fn into(self) -> TagTableRow {
        let mut row = TagTableRow::default();
        row.text = format!("{:010}", self.insert_position);
        row.tag_map.insert(2, vec![self.cncx_offset]);
        row.tag_map.insert(3, vec![self.file_number]);
        row.tag_map.insert(4, vec![self.sequence_number]);
        row.tag_map.insert(6, vec![self.start_offset, self.length]);
        row
    }
}

impl<'a> Index<'a> for ChunkIndex {
    fn get_tag_definitions() -> Vec<TagDefinition> {
        vec![
            TagDefinition {
                tag: 2,
                values_per_entry: 1,
                mask: 1,
                end_flag: 0,
            },
            TagDefinition {
                tag: 3,
                values_per_entry: 1,
                mask: 2,
                end_flag: 0,
            },
            TagDefinition {
                tag: 4,
                values_per_entry: 1,
                mask: 4,
                end_flag: 0,
            },
            TagDefinition {
                tag: 6,
                values_per_entry: 2,
                mask: 8,
                end_flag: 0,
            },
        ]
    }
}

#[derive(Debug, PartialEq)]
pub struct SkeletonIndex {
    pub name: String,
    pub chunk_count: u32,
    pub start_offset: u32,
    pub length: u32,
}

impl<'a> Index<'a> for SkeletonIndex {
    fn get_tag_definitions() -> Vec<TagDefinition> {
        // todo: lazy static
        vec![
            TagDefinition {
                tag: 1,
                values_per_entry: 1,
                mask: 3,
                end_flag: 0,
            },
            TagDefinition {
                tag: 6,
                values_per_entry: 2,
                mask: 12,
                end_flag: 0,
            },
        ]
    }
}

impl<'a> TryFrom<&'a TagTableRow> for SkeletonIndex {
    type Error = TagTableRowParseError;

    fn try_from(row: &TagTableRow) -> Result<Self, Self::Error> {
        let chunk_count = row
            .tag_map
            .get(&1)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("chunk_count".to_string()))?[0];
        let geometry_pair = row
            .tag_map
            .get(&6)
            .ok_or_else(|| TagTableRowParseError::TagNotFound("geometry".to_string()))?;
        let start_offset = geometry_pair[0];
        let length = geometry_pair[1];

        Ok(SkeletonIndex {
            name: row.text.clone(),
            chunk_count,
            start_offset,
            length,
        })
    }
}

impl Into<TagTableRow> for SkeletonIndex {
    fn into(self) -> TagTableRow {
        let mut row = TagTableRow::default();
        row.text = self.name;
        row.tag_map
            .insert(1, vec![self.chunk_count, self.chunk_count]);
        row.tag_map.insert(
            6,
            vec![
                self.start_offset,
                self.length,
                self.start_offset,
                self.length,
            ],
        );
        row
    }
}
