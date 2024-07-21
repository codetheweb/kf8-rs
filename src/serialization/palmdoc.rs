use std::io::{Read, Write};

use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
struct PalmDocRecordOffset {
    offset: u32,
    #[deku(temp, temp_value = "0")]
    _unused_flags: u8,
    #[deku(bytes = "3")]
    #[cfg_attr(test, proptest(strategy = "0..=u32::from(ux::u24::MAX)"))]
    unique_id: u32,
}

fn escape_title(title: &String) -> String {
    title
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect::<String>()
        .replace(' ', "_")
}

fn read_records<R: Read>(
    reader: &mut deku::reader::Reader<R>,
    record_offsets: &Vec<PalmDocRecordOffset>,
) -> Result<Vec<Vec<u8>>, DekuError> {
    let mut records = Vec::new();

    for (start, end) in record_offsets.iter().zip(record_offsets.iter().skip(1)) {
        let len = end.offset as usize - start.offset as usize;
        let mut record = vec![0; len];
        reader.read_bytes(len, &mut record)?;
        records.push(record);
    }

    Ok(records)
}

fn write_records<W: Write>(
    writer: &mut deku::writer::Writer<W>,
    records: &Vec<Vec<u8>>,
) -> Result<(), DekuError> {
    writer.write_bytes(records.concat().as_slice())?;
    Ok(())
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
// todo: should not be Cloneable
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct PalmDoc {
    #[deku(
        reader = "crate::utils::deku::read_string(deku::reader, 32)",
        writer = "crate::utils::deku::write_fixed_length_string(deku::writer, escape_title(title).as_str(), 32)"
    )]
    #[cfg_attr(test, proptest(strategy = "\"[a-zA-Z0-9]{0, 32}\""))]
    pub title: String,
    #[deku(temp, temp_value = "0")]
    _unused_attributes: u16,
    #[deku(temp, temp_value = "0")]
    _unused_version: u16,
    /// seconds since epoch
    pub created_at: u32,
    /// seconds since epoch
    pub modified_at: u32,
    /// seconds since epoch
    pub last_backed_up_at: u32,
    #[deku(temp, temp_value = "0")]
    _unused_modification_number: u32,
    #[deku(temp, temp_value = "0")]
    _unused_app_info_id: u32,
    #[deku(temp, temp_value = "0")]
    _unused_sort_info_id: u32,
    #[deku(temp, temp_value = "*b\"BOOK\"")]
    _unused_type: [u8; 4],
    #[deku(temp, temp_value = "*b\"MOBI\"")]
    _unused_creator: [u8; 4],
    #[deku(temp, temp_value = "(2 * records.len()).saturating_sub(1) as u32")]
    _unused_unique_id_seed: u32,
    #[deku(temp, temp_value = "0")]
    _unused_next_record_list_id: u32,
    #[deku(temp, temp_value = "records.len() as u16")]
    num_records: u16,
    #[deku(temp, count = "num_records")]
    record_metadata: Vec<PalmDocRecordOffset>,
    #[deku(temp, temp_value = "[0; 2]")]
    _padding: [u8; 2],
    #[deku(
        reader = "read_records(deku::reader, record_metadata)",
        writer = "write_records(deku::writer, records)"
    )]
    pub records: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_palmdoc_roundtrip(header in any::<PalmDoc>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        header.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = PalmDoc::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(header, decoded);
      }
    }
}
