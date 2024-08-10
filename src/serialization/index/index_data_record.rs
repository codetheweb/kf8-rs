use std::io::{Cursor, Read, Write};

use deku::prelude::*;

use crate::serialization::{
    tag_map::{TagDefinition, TagMapEntry},
    IndxHeader,
};

use super::types::{IndexTagMapEntry, TagMapEntryParseError};

#[derive(Debug, PartialEq)]
pub struct IndexDataRecord {
    pub header: IndxHeader,
    pub entries: Vec<TagMapEntry>,
}

impl IndexDataRecord {
    pub fn parse_as<'a, T: IndexTagMapEntry<'a>>(
        &'a self,
    ) -> Result<Vec<T>, TagMapEntryParseError> {
        self.entries
            .iter()
            .map(|entry| T::try_from(&entry))
            .collect()
    }
}

impl<'a> DekuReader<'a, &Vec<TagDefinition>> for IndexDataRecord {
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
        let entry_data_length = header.block_offset as usize - header_length;

        let mut entries_buf = Vec::with_capacity(entry_data_length);
        for _ in 0..entry_data_length {
            let mut buf = [0u8];
            reader.read_bytes_const(&mut buf)?;
            entries_buf.push(buf[0]);
        }

        let mut magic = [0u8; 4];
        reader.read_bytes_const(&mut magic)?;
        if magic != *b"IDXT" {
            return Err(DekuError::Parse(
                format!("Invalid magic: {:?}", String::from_utf8_lossy(&magic)).into(),
            ));
        }

        let mut entry_offsets = Vec::new();
        for _ in 0..header.num_entries {
            let mut buf = [0; 2];
            reader.read_bytes(2, &mut buf)?;
            let offset = u16::from_be_bytes(buf);
            entry_offsets.push(offset as usize);
        }

        let mut entries = Vec::new();
        for (beginning_offset, end_offset) in entry_offsets
            .iter()
            .zip(
                entry_offsets
                    .iter()
                    .skip(1)
                    .chain(std::iter::once(&(header.block_offset as usize))),
            )
            .map(|(start, end)| (start - header_length, end - header_length))
        {
            // todo: use single reader/cursor
            let mut cursor = Cursor::new(&entries_buf[beginning_offset..end_offset]);
            let mut reader = Reader::new(&mut cursor);

            let entry = TagMapEntry::from_reader_with_ctx(
                &mut reader,
                ((end_offset - beginning_offset) as usize, tag_definitions),
            )
            .unwrap();

            entries.push(entry);
        }

        Ok(IndexDataRecord { header, entries })
    }
}

impl DekuWriter<&Vec<TagDefinition>> for IndexDataRecord {
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        tag_definitions: &Vec<TagDefinition>,
    ) -> Result<(), DekuError> {
        // todo: bad abstraction/need a better API?
        let mut header = self.header.clone();
        // header.len = 36 + 4 + self.entries.len() as u32 * 2;
        header.num_entries = self.entries.len() as u32;
        let header_len = self.header.to_bytes().unwrap().len();

        let mut serialized_entries = self
            .entries
            .iter()
            .map(|entry| {
                let mut cursor = Cursor::new(Vec::new());
                let mut writer = Writer::new(&mut cursor);
                entry.to_writer(&mut writer, tag_definitions).unwrap();
                writer.finalize().unwrap();
                cursor.into_inner()
            })
            .collect::<Vec<_>>();

        let entry_data_len = serialized_entries
            .iter()
            .map(|entry| entry.len())
            .sum::<usize>();

        // Pad to 4 byte alignment
        let padding = vec![0u8; 4 - (header_len + entry_data_len) % 4];
        serialized_entries.push(padding.clone());

        header.block_offset = header_len as u32
            + serialized_entries
                .iter()
                .map(|entry| entry.len())
                .sum::<usize>() as u32;

        // Write header
        header.to_writer(writer, ())?;

        // Write entries
        for entry in serialized_entries.iter() {
            writer.write_bytes(entry)?;
        }

        // Write magic
        writer.write_bytes(b"IDXT")?;

        // Write entry offsets
        let mut num_written_bytes = 0;
        for offset in serialized_entries.iter().scan(header_len, |offset, entry| {
            let current_offset = *offset;
            *offset += entry.len();
            Some(current_offset)
        }) {
            writer.write_bytes(&((offset as u16).to_be_bytes()))?;
            num_written_bytes += 2;
        }
        // Pad to 4 byte alignment
        let padding = vec![0u8; 4 - (num_written_bytes % 4)];
        writer.write_bytes(&padding)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::serialization::{ChunkTagMapEntry, SkeletonTagMapEntry};

    use super::*;
    use proptest::prelude::*;

    fn arbitrary_index_data_record() -> impl Strategy<Value = (IndexDataRecord, Vec<TagDefinition>)>
    {
        any::<bool>()
            .prop_map(|generate_skeleton_entries| {
                let definitions = match generate_skeleton_entries {
                    true => SkeletonTagMapEntry::get_tag_definitions(),
                    false => ChunkTagMapEntry::get_tag_definitions(),
                };

                let entries = match generate_skeleton_entries {
                    true => proptest::collection::vec(
                        any::<SkeletonTagMapEntry>().prop_map(|v| {
                            let entry: TagMapEntry = v.into();
                            entry
                        }),
                        0..=128, // 128 is arbitrary
                    )
                    .boxed(),
                    false => proptest::collection::vec(
                        any::<ChunkTagMapEntry>().prop_map(|v| {
                            let entry: TagMapEntry = v.into();
                            entry
                        }),
                        0..=128, // 128 is arbitrary
                    )
                    .boxed(),
                };

                (definitions, entries)
            })
            .prop_flat_map(|(definitions, entries)| {
                let header = any::<IndxHeader>();

                (Just(definitions), header, entries)
            })
            .prop_map(|(definitions, header, entries)| {
                let record = IndexDataRecord { header, entries };

                (record, definitions)
            })
    }

    proptest! {
      #[test]
      fn test_index_data_record_roundtrip((record, definitions) in arbitrary_index_data_record()) {
        env_logger::try_init();
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        record.to_writer(&mut writer, &definitions).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = IndexDataRecord::from_reader_with_ctx(&mut reader, &definitions).unwrap();

        assert_eq!(record.entries, decoded.entries); // todo: should compare entire record once abstraction is fixed
      }
    }
}
