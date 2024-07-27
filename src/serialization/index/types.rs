use std::{collections::HashMap, io::Read, io::Write};

use deku::prelude::*;
use nom::{bytes::complete::take, error::ErrorKind, number::complete::be_u8, IResult};
use thiserror::Error;

use crate::{serialization::TagDefinition, tag_map::parse_tag_map};

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
// todo: move TagDefinition?
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

pub trait IndexRow<'a>:
    TryFrom<&'a TagTableRow, Error = TagTableRowParseError> + Into<TagTableRow>
{
    fn get_tag_definitions() -> Vec<TagDefinition>;
}
