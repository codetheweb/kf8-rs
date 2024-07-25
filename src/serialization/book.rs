use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    time::{SystemTime, UNIX_EPOCH},
    u32,
};

use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::constants::{MainLanguage, SubLanguage};

use super::{
    exth::Exth, BookType, Codepage, CompressionType, ExthFlags, ExtraDataFlags, FDSTTable,
    LanguageCode, MobiHeader, PalmDoc,
};

const TEXT_RECORD_SIZE: usize = 4096; // todo: assert that chunks are this length?

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Book {
    pub title: String,
    pub uid: u32,
    pub main_language: Option<MainLanguage>,
    pub sub_language: Option<SubLanguage>,
    pub text: String,
}

impl TryFrom<PalmDoc> for Book {
    type Error = DekuError;

    fn try_from(palmdoc: PalmDoc) -> Result<Self, Self::Error> {
        let first_record = palmdoc
            .records
            .first()
            .ok_or(DekuError::Parse("No records".into()))?;
        let (_, mobi_header) = MobiHeader::from_bytes((first_record, 0))?;

        let mut text = Vec::new();
        for i in 1..(mobi_header.last_text_record + 1) as usize {
            let record = &palmdoc
                .records
                .get(i)
                .ok_or(DekuError::Parse("No records".into()))?;

            let record_data =
                &record[0..record.len() - mobi_header.sizeof_trailing_section_entries(record)];

            match mobi_header.compression_type {
                CompressionType::None => {
                    text.extend_from_slice(record_data);
                }
                CompressionType::HuffCdic => {
                    todo!()
                }
                CompressionType::PalmDoc => {
                    let decompressed = palmdoc_compression::decompress(record_data)
                        .map_err(|_| DekuError::Parse("Failed to decompress".into()))?;
                    text.extend_from_slice(&decompressed);
                }
            }
        }

        Ok(Book {
            title: mobi_header.title,
            uid: mobi_header.uid,
            main_language: mobi_header.language_code.main,
            sub_language: mobi_header.language_code.sub,
            text: String::from_utf8(text).map_err(|_| DekuError::Parse("Invalid UTF-8".into()))?,
        })
    }
}

// todo: cleaner?
fn create_text_record(text: &mut Cursor<&[u8]>) -> (Vec<u8>, Vec<u8>) {
    let opos = text.position();
    let npos = std::cmp::min(
        (opos + TEXT_RECORD_SIZE as u64) as u64,
        text.get_ref().len() as u64,
    );
    let mut extra = 0;

    let mut last = Vec::new();
    while std::str::from_utf8(&last).is_err() {
        let size = last.len() + 1;
        text.seek(SeekFrom::Start(npos - size as u64)).unwrap();
        last.clear();
        last.resize(size, 0);
        text.read_exact(&mut last).unwrap();
    }

    if std::str::from_utf8(&last).is_err() {
        let mut prev = last.len();
        loop {
            text.seek(SeekFrom::Start(npos - prev as u64)).unwrap();
            last.resize(last.len() + 1, 0);
            text.read_exact(&mut last[prev..]).unwrap();
            if std::str::from_utf8(&last).is_ok() {
                break;
            }
            prev += 1;
        }
        extra = last.len() - prev;
    }

    text.seek(SeekFrom::Start(opos)).unwrap();
    let this_record_size = (npos - opos) as usize;
    let mut data = vec![0; this_record_size];
    text.read_exact(&mut data).unwrap();
    let mut overlap = vec![0; extra];
    text.read_exact(&mut overlap).unwrap();
    text.seek(SeekFrom::Start(npos)).unwrap();

    (data, overlap)
}

const FLIS: &[u8; 36] = b"FLIS\0\0\0\x08\0\x41\0\0\0\0\0\0\xff\xff\xff\xff\0\x01\0\x03\0\0\0\x03\0\0\0\x01\xff\xff\xff\xff";

fn create_fcis_record(text_length: usize) -> Vec<u8> {
    let mut fcis: Vec<u8> = vec![
        0x46, 0x43, 0x49, 0x53, // 'FCIS'
        0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x00,
    ];
    fcis.extend_from_slice(&text_length.to_be_bytes());
    fcis.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x28, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ]);

    fcis
}

impl TryFrom<&Book> for PalmDoc {
    type Error = DekuError;

    fn try_from(book: &Book) -> Result<Self, Self::Error> {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("couldn't get time since epoch");
        // todo: allow customizing? builder pattern?
        let created_at_seconds = since_the_epoch.as_secs();

        let mut records = vec![];

        // Text records
        let mut text_cursor = Cursor::new(book.text.as_bytes());
        while text_cursor.position() < text_cursor.get_ref().len() as u64 {
            let (record, mut overlap) = create_text_record(&mut text_cursor);
            // todo: make compression configurable?
            let mut compressed_record = palmdoc_compression::compress(&record);
            compressed_record.append(&mut overlap);
            compressed_record.push(overlap.len() as u8);
            records.push(compressed_record);
        }

        let last_text_record = records.len();
        let mut first_non_text_record = last_text_record + 1;

        // Pad to 4 bytes
        let records_data_len = records.iter().map(|r| r.len()).sum::<usize>();
        if records_data_len % 4 != 0 {
            let padding = 4 - (records_data_len % 4);
            records.push(vec![0; padding]);
            first_non_text_record += 1;
        }

        // Metadata records
        let chunk_index = u32::MAX; //records.len();
                                    // todo: add chunks to records
        let skel_index = u32::MAX; //records.len(); // todo: rename
                                   // todo: add skel to records

        let guide_index = u32::MAX; // todo
        let ncx_index = u32::MAX; // todo

        // Resource records
        // todo

        // FDST
        let fdst_record = records.len();
        records.push(FDSTTable { entries: vec![] }.to_bytes().unwrap());

        // FCIS
        let fcis_record = records.len();
        records.push(create_fcis_record(book.text.len()));

        // FLIS
        let flis_record = records.len();
        records.push(FLIS.to_vec());

        // EOF
        records.push(b"\xe9\x8e\r\n".to_vec());

        // todo: consistent terms between _record and _index
        let mobi_header = MobiHeader {
            title: book.title.clone(),
            compression_type: CompressionType::PalmDoc,
            text_length: book.text.len() as u32,
            last_text_record: last_text_record as u16,
            text_record_size: TEXT_RECORD_SIZE as u16,
            book_type: BookType::Book,
            text_encoding: Codepage::Utf8,
            uid: book.uid,
            file_version: 8,
            first_non_text_record: first_non_text_record as u32,
            title_offset: 0, // todo: ?
            language_code: LanguageCode {
                main: book.main_language.clone(),
                sub: book.sub_language.clone(),
            },
            first_resource_record: u32::MAX, // todo
            exth_flags: ExthFlags {
                has_exth: false, // todo
                has_fonts: false,
                is_periodical: false,
            },
            fdst_record: fdst_record as u32,
            fdst_count: fdst_record as u32,
            fcis_record: fcis_record as u32,
            fcis_count: 1,
            flis_record: flis_record as u32,
            flis_count: 1,
            srcs_record: u32::MAX,
            srcs_count: 0,
            extra_data_flags: ExtraDataFlags {
                extra_multibyte_bytes_after_text_records: true,
                has_tbs: false,
                uncrossable_breaks: false,
            },
            ncx_index,
            chunk_index: chunk_index as u32,
            skel_index: skel_index as u32,
            datp_index: u32::MAX,
            guide_index,
            exth: None, // todo
        };
        records.insert(0, mobi_header.to_bytes()?);

        Ok(PalmDoc {
            title: book.title.clone(),
            created_at: created_at_seconds as u32,
            modified_at: created_at_seconds as u32,
            last_backed_up_at: 0,
            records,
        })
    }
}

// todo: should this be DekuContainerReader?
impl<'a, Ctx> DekuReader<'a, Ctx> for Book {
    fn from_reader_with_ctx<R: Read>(reader: &mut Reader<R>, ctx: Ctx) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let palmdoc = PalmDoc::from_reader_with_ctx(reader, ())?;
        let book = Book::try_from(palmdoc)?;
        Ok(book)
    }
}

impl<'a, Ctx> DekuWriter<Ctx> for Book {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _ctx: Ctx) -> Result<(), DekuError> {
        let palmdoc = PalmDoc::try_from(self)?;
        palmdoc.to_writer(writer, ())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{arbitrary::any, proptest};

    // todo: rename
    #[test]
    fn parse_deku() {
        // todo
        env_logger::init();
        let mut file = std::fs::File::open("resources/war_and_peace.azw3").unwrap();
        let mut reader = Reader::new(&mut file);
        // let mut expected_html_reader =
        //     std::fs::File::open("resources/war_and_peace.rawml").unwrap();
        // let mut expected_html = String::new();
        // expected_html_reader
        //     .read_to_string(&mut expected_html)
        //     .unwrap();

        let book = Book::from_reader_with_ctx(&mut reader, ()).unwrap();
        println!("{:?}", book);
    }

    proptest! {
        #[test]
        fn test_book_roundtrip(book in any::<Book>()) {
            env_logger::try_init();

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            book.to_writer(&mut writer, ()).unwrap();
            writer.finalize().unwrap();

            serialized.set_position(0);

            let mut reader = Reader::new(&mut serialized);
            let parsed = Book::from_reader_with_ctx(&mut reader, ()).unwrap();

            assert_eq!(book, parsed);
        }
    }
}
