use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[deku_derive(DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian", endian = "endian")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Record {
    pub offset: u32,
    #[deku(temp, temp_value = "0")]
    _unused_flags: u8,
    #[deku(bytes = "3")]
    #[cfg_attr(test, proptest(strategy = "0..=u32::from(ux::u24::MAX)"))]
    pub unique_id: u32,
}

fn escape_title(title: &String) -> String {
    title
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect::<String>()
        .replace(' ', "_")
}

#[deku_derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct PalmDocHeader {
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
    #[deku(count = "num_records")]
    pub records: Vec<Record>,
    #[deku(temp, temp_value = "[0; 2]")]
    _padding: [u8; 2],
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_palmdoc_header_roundtrip(header in any::<PalmDocHeader>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        header.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = PalmDocHeader::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(header, decoded);
      }
    }
}
