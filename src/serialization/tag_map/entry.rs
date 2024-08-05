use std::{collections::HashMap, io::Read, io::Write};

use cookie_factory::gen;
use deku::prelude::*;
use nom::{bytes::complete::take, error::ErrorKind, number::complete::be_u8, IResult};

use super::{read_entry::parse_tag_map, write_entry::serialize_tag_map, TagDefinition};

#[derive(Debug, PartialEq)]
pub struct TagMapEntry {
    pub text: String,
    pub tag_map: HashMap<u8, Vec<u32>>,
}

impl Default for TagMapEntry {
    fn default() -> Self {
        TagMapEntry {
            text: "".to_string(),
            tag_map: HashMap::new(),
        }
    }
}

fn read_tag_map_entry<'a>(
    input: &'a [u8],
    definitions: &Vec<TagDefinition>,
) -> IResult<&'a [u8], TagMapEntry> {
    let (input, text_length) = be_u8(input)?;
    let (input, text) = take(text_length as usize)(input)?;
    // remap error
    let text = String::from_utf8(text.to_vec())
        .map_err(|_| nom::Err::Error(nom::error::make_error(input, ErrorKind::IsNot)))?;

    let (_, tag_map) = parse_tag_map(definitions, input)?;

    Ok((input, TagMapEntry { text, tag_map }))
}

// ctx is byte length of the entry
// todo: move TagDefinition?
impl<'a> DekuReader<'a, (usize, &Vec<TagDefinition>)> for TagMapEntry {
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        ctx: (usize, &Vec<TagDefinition>),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let (length, definitions) = ctx;
        let mut buf = vec![0; length];
        reader.read_bytes(length, &mut buf)?;
        let (_, entry) = read_tag_map_entry(&buf, definitions).unwrap();
        Ok(entry)
    }
}

impl DekuWriter<&Vec<TagDefinition>> for TagMapEntry {
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        definitions: &Vec<TagDefinition>,
    ) -> Result<(), DekuError> {
        writer.write_bytes(&[self.text.len() as u8])?;
        writer.write_bytes(self.text.as_bytes())?;

        let mut buf = Vec::new();
        gen(serialize_tag_map(definitions, &self.tag_map), &mut buf).unwrap();
        writer.write_bytes(&buf)?;

        Ok(())
    }
}
