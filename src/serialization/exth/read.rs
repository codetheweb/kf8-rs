use std::collections::HashMap;

use nom::{
    bytes::complete::take,
    multi::count,
    number::complete::{be_u16, be_u32, be_u8},
    IResult,
};

use crate::{
    constants::{MetadataId, MetadataIdValue},
    ExthKeyValue,
};

fn read_exth_key_value(input: &[u8]) -> IResult<&[u8], ExthKeyValue> {
    let (input, id) = nom::number::complete::be_u32(input)?;

    let (input, content_len) = be_u32(input)?;
    let (input, content) = take(content_len as usize - 8)(input)?;

    if let Ok(id) = MetadataId::try_from(id) {
        let parsed = String::from_utf8(content.to_vec()).unwrap();

        return Ok((input, ExthKeyValue::ID(id, parsed)));
    } else if let Ok(id) = MetadataIdValue::try_from(id) {
        let value: u32 = match content_len {
            9 => be_u8(content)?.1 as u32,
            10 => be_u16(content)?.1 as u32,
            12 => be_u32(content)?.1 as u32,
            _ => panic!(),
        };

        return Ok((input, ExthKeyValue::Value(id, value)));
    }

    // todo: don't panic
    panic!()
}

pub(super) fn read_exth(
    input: &[u8],
) -> IResult<
    &[u8],
    (
        HashMap<MetadataId, Vec<String>>,
        HashMap<MetadataIdValue, Vec<u32>>,
    ),
> {
    let (input, num_items) = be_u32(input)?;

    let (input, items) = count(read_exth_key_value, num_items as usize)(input)?;

    let mut standard_metadata: HashMap<MetadataId, Vec<String>> = HashMap::new();
    let mut kf8_metadata: HashMap<MetadataIdValue, Vec<u32>> = HashMap::new();
    for item in items {
        match item {
            ExthKeyValue::ID(id, content) => standard_metadata.entry(id).or_default().push(content),
            ExthKeyValue::Value(id, data) => kf8_metadata.entry(id).or_default().push(data),
        }
    }

    Ok((input, (standard_metadata, kf8_metadata)))
}
