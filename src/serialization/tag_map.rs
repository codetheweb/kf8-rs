use cookie_factory::multi;
use cookie_factory::sequence::tuple;
use cookie_factory::{bytes::be_u8, SerializeFn};
use std::collections::HashMap;
use std::io::Write;

use crate::utils::deku::cookie_write_variable_width_value;

use super::TagDefinition;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MASK_TO_BIT_SHIFTS: HashMap<u8, u32> = {
        let mut m = HashMap::new();
        m.insert(1, 0);
        m.insert(2, 1);
        m.insert(3, 0);
        m.insert(4, 2);
        m.insert(8, 3);
        m.insert(12, 2);
        m.insert(16, 4);
        m.insert(32, 5);
        m.insert(48, 4);
        m.insert(64, 6);
        m.insert(128, 7);
        m.insert(192, 6);
        m
    };
}

fn write_control_byte<W: Write>(
    num_entries: usize,
    definitions: &Vec<TagDefinition>,
) -> impl SerializeFn<W> {
    let mut control_byte = 0;

    for definition in definitions {
        if definition.end_flag == 1 {
            break;
        }

        let num_entries = num_entries / definition.values_per_entry as usize;
        let shifts = MASK_TO_BIT_SHIFTS.get(&definition.mask).unwrap();
        control_byte |= definition.mask & (num_entries << shifts) as u8;
    }

    be_u8(control_byte)
}

pub fn serialize_tag_map<'a, W: Write + 'a>(
    tag_table: &'a Vec<TagDefinition>,
    tag_map: &'a HashMap<u8, Vec<u32>>,
) -> impl SerializeFn<W> + 'a {
    multi::all(tag_table.iter().map(move |definition| {
        // todo: gross
        let values_default = vec![];
        let values = tag_map.get(&definition.tag).unwrap_or(&values_default);
        let values = values.clone();

        tuple((
            write_control_byte(values.len(), &tag_table),
            multi::all(values.into_iter().map(move |value| {
                cookie_write_variable_width_value(value, deku::ctx::Endian::Big)
            })),
        ))
    }))
}
