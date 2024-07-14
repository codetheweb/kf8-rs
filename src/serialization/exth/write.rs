use std::io::Write;

use cookie_factory::{
    bytes::{be_u16, be_u32, be_u8},
    combinator::slice,
    multi,
    sequence::tuple,
    SerializeFn,
};

use crate::constants::{MetadataId, MetadataIdValue};

fn write_exth_key_value_id<'a, W: Write + 'a>(
    id: &'a MetadataId,
    content: &'a String,
) -> Box<dyn SerializeFn<W> + 'a> {
    let id: u32 = id.clone().into();
    let content = content.as_bytes();
    let content_len = content.len() as u32 + 8;

    Box::new(tuple((be_u32(id), be_u32(content_len), slice(content))))
}

fn write_exth_key_value_value<'a, W: Write + 'a>(
    id: &'a MetadataIdValue,
    value: &'a u32,
) -> Box<dyn SerializeFn<W> + 'a> {
    let id: u32 = id.clone().into();

    let mut content_len = 9;
    let mut value_writer: Box<dyn SerializeFn<W>> = Box::new(be_u8::<W>(*value as u8));

    if (u8::MAX as u32) < *value && *value <= u16::MAX as u32 {
        content_len = 10;
        value_writer = Box::new(be_u16::<W>(*value as u16));
    } else if (u16::MAX as u32) < *value && *value <= u32::MAX {
        content_len = 12;
        value_writer = Box::new(be_u32::<W>(*value));
    }

    // todo: box necessary?
    Box::new(tuple((be_u32(id), be_u32(content_len), value_writer)))
}

pub fn write_exth<'a, W: Write + 'a>(exth: &'a super::Exth) -> impl SerializeFn<W> + 'a {
    let num_items = exth.metadata_id.values().map(|v| v.len()).sum::<usize>()
        + exth.metadata_value.values().map(|v| v.len()).sum::<usize>();

    tuple((
        be_u32(num_items as u32),
        multi::all(exth.metadata_id.iter().map(|(id, values)| {
            multi::all(
                values
                    .iter()
                    .map(|value| write_exth_key_value_id(id, value)),
            )
        })),
        multi::all(exth.metadata_value.iter().map(|(id, values)| {
            multi::all(
                values
                    .iter()
                    .map(|value| write_exth_key_value_value(id, value)),
            )
        })),
    ))
}
