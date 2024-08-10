use deku::prelude::*;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use std::collections::HashMap;

fn align_bytes(mut bytes: Vec<u8>, alignment: usize) -> Vec<u8> {
    let padding = alignment - (bytes.len() % alignment);
    bytes.extend(vec![0; padding]);
    bytes
}

const MAX_STRING_LENGTH: usize = 500;
const MAX_RECORD_LENGTH: usize = 0x10000 - 1024; // kindlegen appears to use 1024, PDB limit is 0x10000

#[deku_derive(DekuRead, DekuWrite)]
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
struct SerializedString {
    #[deku(
        temp,
        reader = "crate::utils::deku::read_big_endian_variable_width_value(deku::reader)",
        writer = "crate::utils::deku::write_variable_width_value(deku::writer, value.len() as u32, deku::ctx::Endian::Big)"
    )]
    len: u32,
    #[deku(
        reader = "crate::utils::deku::read_string(deku::reader, *len as usize)",
        writer = "crate::utils::deku::write_string(deku::writer, value)"
    )]
    value: String,
}

impl SerializedString {
    fn new(value: String) -> Self {
        SerializedString { value }
    }
}

// todo: rename?
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct CNCXRecords {
    #[cfg_attr(
        test,
        proptest(
            // Non-empty, unique strings. Arbitrary limit of 256 strings to speed up the test.
            strategy = "proptest::collection::vec(\"[a-zA-Z0-9]{1,500}\", 0..=256).prop_map(|strings| {
              let mut seen = std::collections::HashSet::new();
              strings.into_iter().filter(|s| seen.insert(s.clone())).collect()
            })"
        )
    )]
    pub strings: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct SerializedCNCXRecords {
    pub records: Vec<Vec<u8>>,
    pub offsets: HashMap<String, usize>,
}

impl CNCXRecords {
    pub fn to_records(self) -> SerializedCNCXRecords {
        let mut records = Vec::new();
        let mut offsets = HashMap::new();

        let mut current_record = Vec::new();
        let mut current_offset = 0;

        for string in self.strings {
            // todo: handle string length overflow
            let mut serialized = SerializedString::new(string.clone()).to_bytes().unwrap();

            if current_record.len() + serialized.len() > MAX_RECORD_LENGTH {
                records.push(align_bytes(current_record, 4));
                current_record = Vec::new();
                current_offset = records.len() * 0x10000;
            }

            current_offset += serialized.len();
            current_record.append(&mut serialized);
            offsets.insert(string, current_offset);
        }

        if current_record.len() > 0 {
            records.push(align_bytes(current_record, 4));
        }

        SerializedCNCXRecords { records, offsets }
    }

    pub fn from_records(serialized: &SerializedCNCXRecords) -> Self {
        let mut strings = Vec::new();

        for record in &serialized.records {
            let mut offset = 0;
            while offset < record.len() {
                if record[offset] == 0 {
                    break;
                }

                let ((leftover, _), serialized_string) =
                    SerializedString::from_bytes((&record[offset..], 0)).unwrap();

                offset += record.len() - offset - leftover.len();
                strings.push(serialized_string.value);
            }
        }

        CNCXRecords { strings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    proptest! {
        #[test]
        fn test_cncx_records_roundtrip(records in any::<CNCXRecords>()) {
            env_logger::try_init();
            let serialized = records.clone().to_records();
            let decoded = CNCXRecords::from_records(&serialized);

            assert_eq!(records, decoded);
        }
    }
}
