use std::io::{Cursor, Read};

use deku::prelude::*;

use crate::serialization::{IndxHeader, TagDefinition};

use super::types::{IndexRow, TagTableRow, TagTableRowParseError};

pub struct IndexDataRecord {
    header: IndxHeader,
    pub rows: Vec<TagTableRow>,
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
        // todo: where does +4 come from?
        let row_data_length = header.block_offset as usize - header_length + 4;

        let mut rows_buf = Vec::with_capacity(row_data_length);
        for _ in 0..row_data_length {
            let mut buf = [0u8];
            reader.read_bytes_const(&mut buf)?;
            rows_buf.push(buf[0]);
        }

        let mut row_offsets = Vec::new();
        for _ in 0..header.num_entries {
            let mut buf = [0; 2];
            reader.read_bytes(2, &mut buf)?;
            let offset = u16::from_be_bytes(buf);
            row_offsets.push(offset as usize);
        }

        let mut rows = Vec::new();

        for (beginning_offset, end_offset) in row_offsets
            .iter()
            .zip(
                row_offsets
                    .iter()
                    .skip(1)
                    .chain(std::iter::once(&(header.block_offset as usize))),
            )
            .map(|(start, end)| (start - header_length, end - header_length))
        {
            // todo: use single reader/cursor
            let mut cursor = Cursor::new(&rows_buf[beginning_offset..end_offset]);
            let mut reader = Reader::new(&mut cursor);

            let row = TagTableRow::from_reader_with_ctx(
                &mut reader,
                ((end_offset - beginning_offset) as usize, tag_definitions),
            )
            .unwrap();

            rows.push(row);
        }

        Ok(IndexDataRecord { header, rows })
    }
}

impl IndexDataRecord {
    pub fn parse_as<'a, T: IndexRow<'a>>(&'a self) -> Result<Vec<T>, TagTableRowParseError> {
        self.rows.iter().map(|row| T::try_from(&row)).collect()
    }
}
