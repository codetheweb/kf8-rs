use deku::reader::Reader;
use nom::{
    bytes::complete::take, combinator::peek, multi::count, number::complete::be_u8, IResult,
};
use std::{collections::HashMap, io::Cursor};

use crate::{serialization::TagDefinition, utils::deku::read_big_endian_variable_width_value};

// Decode variable width value from given bytes.
fn get_variable_width_value(data: &[u8]) -> IResult<&[u8], u32> {
    let mut reader = Cursor::new(data);
    let mut reader = Reader::new(&mut reader);
    let value = read_big_endian_variable_width_value(&mut reader).unwrap();
    let consumed = reader.into_inner().position() as usize;

    Ok((&data[consumed..], value))
}

// Create a map of tags and values from the given byte section.
pub fn parse_tag_map<'a>(
    definitions: &Vec<TagDefinition>,
    data: &'a [u8],
) -> IResult<&'a [u8], HashMap<u8, Vec<u32>>> {
    // let (mut remaining, _) = take(control_byte_count)(data)?;
    let mut remaining = data;

    #[derive(Debug)]
    struct SingleTagHeader {
        tag: u8,
        value_count: Option<u8>,
        value_bytes: Option<u32>,
        values_per_entry: u8,
    }

    let mut tag_headers: Vec<SingleTagHeader> = vec![];

    // todo: more idiomatic
    for table_entry in definitions {
        let TagDefinition {
            tag,
            values_per_entry,
            mask,
            end_flag,
        } = table_entry;
        if *end_flag == 0x01 {
            let (r, _) = take(1usize)(remaining)?;
            remaining = r;
            continue;
        }

        let (mut rem, control_byte) = peek(be_u8)(remaining)?;
        let mut value = control_byte & *mask;
        if value != 0 {
            if value == *mask {
                if mask.count_ones() > 1 {
                    let (r, value) = get_variable_width_value(rem)?;
                    rem = r;
                    tag_headers.push(SingleTagHeader {
                        tag: *tag,
                        value_count: None,
                        value_bytes: Some(value),
                        values_per_entry: *values_per_entry,
                    })
                } else {
                    tag_headers.push(SingleTagHeader {
                        tag: *tag,
                        value_count: Some(1),
                        value_bytes: None,
                        values_per_entry: *values_per_entry,
                    })
                }
            } else {
                let mut mask = *mask;
                while mask & 0x01 == 0 {
                    mask >>= 1;
                    value >>= 1;
                }

                tag_headers.push(SingleTagHeader {
                    tag: *tag,
                    value_count: value.into(),
                    value_bytes: None,
                    values_per_entry: *values_per_entry,
                })
            }

            remaining = rem;
        }
    }

    let mut tag_hash_map = HashMap::new();

    for tag_header in tag_headers {
        // todo: can remove this
        let mut values = vec![];
        if let Some(value_count) = tag_header.value_count {
            let (r, v) = count(
                get_variable_width_value,
                (value_count * tag_header.values_per_entry) as usize,
            )(remaining)?;
            remaining = r;
            values.extend(v);
        } else if let Some(value_bytes) = tag_header.value_bytes {
            let mut total_consumed = 0usize;
            while total_consumed < value_bytes as usize {
                let (r, value) = get_variable_width_value(remaining)?;
                total_consumed += remaining.len() - r.len();
                remaining = r;
                values.push(value);
            }
        } else {
            panic!("unexpected")
        }

        tag_hash_map.insert(tag_header.tag, values);
    }

    // todo: fix
    Ok((remaining, tag_hash_map))
}
