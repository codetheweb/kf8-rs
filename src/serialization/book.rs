use byteorder::WriteBytesExt;
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    iter::once,
    time::{SystemTime, UNIX_EPOCH},
    u32, vec,
};

use binrw::{BinRead, BinWrite};
use deku::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::{
    constants::{MainLanguage, MetadataId, SubLanguage},
    serialization::{tag_map::TagMapEntry, FDSTEntry, SkeletonTagMapEntry, TotalIndexEntry},
};

use super::{
    exth::Exth, BookType, ChunkTagMapEntry, Codepage, CompressionType, ExthFlags, ExtraDataFlags,
    FDSTTable, LanguageCode, MobiHeader, PalmDoc,
};
use crate::serialization::index::types::IndexTagMapEntry;

const TEXT_RECORD_SIZE: usize = 4096; // todo: assert that chunks are this length?

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BookPart {
    pub skeleton_head: String,
    pub content: String,
    pub skeleton_tail: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Book {
    pub title: String,
    pub uid: u32,
    pub main_language: Option<MainLanguage>,
    pub sub_language: Option<SubLanguage>,
    pub book_parts: Vec<BookPart>,
    pub resources: Vec<String>, // todo
    pub compression: CompressionType,
}

impl TryFrom<PalmDoc> for Book {
    type Error = DekuError;

    fn try_from(palmdoc: PalmDoc) -> Result<Self, Self::Error> {
        let first_record = palmdoc
            .records
            .first()
            .ok_or(DekuError::Parse("No records".into()))?;
        let mobi_header = MobiHeader::read(&mut Cursor::new(first_record)).unwrap();

        let mut text = Vec::new();
        for i in 1..(mobi_header.num_of_text_records + 1) as usize {
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
            title: mobi_header.title.try_into().unwrap(),
            uid: mobi_header.uid,
            main_language: mobi_header.language_code.main,
            sub_language: mobi_header.language_code.sub,
            book_parts: vec![], // todo
            resources: vec![],  // todo
            compression: mobi_header.compression_type,
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
    let mut fcis = vec![
        0x46, 0x43, 0x49, 0x53, // 'FCIS'
        0x00, 0x00, 0x00, 0x14, // 0x14
        0x00, 0x00, 0x00, 0x10, // 0x10
        0x00, 0x00, 0x00, 0x02, // 0x02
        0x00, 0x00, 0x00, 0x00, // 0x00
    ];

    // Pack text_length as big-endian u32
    fcis.write_u32::<byteorder::BigEndian>(text_length as u32)
        .unwrap();

    fcis.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, // 0x00
        0x00, 0x00, 0x00, 0x28, // 0x28
        0x00, 0x00, 0x00, 0x00, // 0x00
        0x00, 0x00, 0x00, 0x28, // 0x28
        0x00, 0x00, 0x00, 0x08, // 0x08
        0x00, 0x01, 0x00, 0x01, // 0x01
        0x00, 0x00, 0x00, 0x00, // 0x00
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

        // Placeholder for header (having a placeholder here allows us to easily calculate record offsets without adding +1 everywhere).
        records.push(vec![]);

        // Text records
        let text_parts_iter = book
            .book_parts
            .iter()
            .map(|part| {
                format!(
                    "{}{}{}",
                    part.skeleton_head, part.skeleton_tail, part.content
                )
            })
            .chain(book.resources.iter().map(|r| r.to_string()));

        let mut text = "".to_string();
        let mut fdst_entries: Vec<FDSTEntry> = vec![];

        let mut pos = 0;
        for part in text_parts_iter {
            let part_len = part.len() as u32;
            fdst_entries.push(FDSTEntry {
                start: pos,
                end: pos + part_len,
            });
            pos += part_len;
            text.push_str(&part);
        }

        let mut text_cursor = Cursor::new(text.as_bytes());
        while text_cursor.position() < text_cursor.get_ref().len() as u64 {
            let (record, mut overlap) = create_text_record(&mut text_cursor);

            match book.compression {
                CompressionType::PalmDoc => {
                    let mut compressed_record = palmdoc_compression::compress(&record);
                    compressed_record.append(&mut overlap);
                    compressed_record.push(overlap.len() as u8);
                    records.push(compressed_record);
                }
                CompressionType::HuffCdic => {
                    todo!()
                }
                CompressionType::None => {
                    let mut record = record;
                    record.append(&mut overlap);
                    record.push(overlap.len() as u8);
                    records.push(record);
                }
            }
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
        let chunk_index_num = records.len();

        // Chunk index
        let chunk_index_entries = book
            .book_parts
            .iter()
            .map(|part| {
                ChunkTagMapEntry {
                    insert_position: part.skeleton_head.len() as u32 - 1, // todo?
                    cncx_offset: 0,
                    file_number: 0,
                    sequence_number: 0,
                    start_offset: 0,
                    length: part.content.len() as u32,
                }
                .into()
            })
            .collect::<Vec<TagMapEntry>>();

        let chunk_index =
            TotalIndexEntry::new(ChunkTagMapEntry::get_tag_definitions(), chunk_index_entries);
        records.extend(chunk_index.into_records());

        let skeleton_index_num = records.len();

        // Skeleton index
        let skeleton_index_entries = book
            .book_parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                SkeletonTagMapEntry {
                    name: "SKEL0000000".to_string(),
                    chunk_count: 1,
                    start_offset: 0,
                    length: part.skeleton_head.len() as u32 + part.skeleton_tail.len() as u32,
                }
                .into()
            })
            .collect::<Vec<TagMapEntry>>();

        let skeleton_index = TotalIndexEntry::new(
            SkeletonTagMapEntry::get_tag_definitions(),
            skeleton_index_entries,
        );
        records.extend(skeleton_index.into_records());

        let guide_index = u32::MAX; // todo
        let ncx_index = u32::MAX; // todo

        // Resource records
        // todo

        // FDST
        let fdst_record = records.len();
        records.push(
            FDSTTable {
                entries: fdst_entries,
            }
            .to_bytes()
            .unwrap(),
        );

        // FLIS
        let flis_record = records.len();
        records.push(FLIS.to_vec());

        // FCIS
        let fcis_record = records.len();
        records.push(create_fcis_record(text.len()));

        // EOF
        records.push(b"\xe9\x8e\r\n".to_vec());

        let mut exth = Exth::default();
        exth.metadata_id.insert(
            MetadataId::Source,
            vec!["calibre:c64482f4-2952-4f2c-ae28-b109cb70f5bb".into()],
        );
        exth.metadata_id.insert(
            MetadataId::Contributor,
            vec!["calibre (7.16.0) [http://calibre-ebook.com]".into()],
        );
        exth.metadata_id
            .insert(MetadataId::UpdatedTitle, vec![book.title.clone()]);
        exth.metadata_id.insert(
            MetadataId::ASIN,
            vec!["c64482f4-2952-4f2c-ae28-b109cb70f5bb".into()],
        );
        exth.metadata_id
            .insert(MetadataId::CdeType, vec!["EBOK".into()]);
        // exth.metadata_id
        //     .insert(MetadataId::CreatorBuildTag, vec!["0730-890adc2".into()]);
        exth.metadata_id
            .insert(MetadataId::ContentLanguageTag, vec!["en".into()]);
        exth.metadata_id.insert(
            MetadataId::Published,
            vec!["2024-08-13T04:05:03.140745+00:00".into()],
        );
        exth.metadata_id
            .insert(MetadataId::OverrideKindleFonts, vec!["true".into()]);
        exth.metadata_id
            .insert(MetadataId::Creator, vec!["kindle".into()]);

        // todo: these aren't serialized correctly?
        // exth.metadata_value
        //     .insert(MetadataIdValue::CreatorSoftware, vec![202]);
        // exth.metadata_value
        //     .insert(MetadataIdValue::CreatorMajorVersion, vec![2]);
        // exth.metadata_value
        //     .insert(MetadataIdValue::CreatorMinorVersion, vec![9]);
        // exth.metadata_value
        //     .insert(MetadataIdValue::CreatorBuildNumber, vec![0]);
        // exth.metadata_value
        //     .insert(MetadataIdValue::EmbeddedRecordCount, vec![0]);

        // todo: consistent terms between _record and _index
        let mobi_header = MobiHeader {
            title: book.title.clone().into(),
            compression_type: CompressionType::None, //CompressionType::PalmDoc,
            text_length: text.len() as u32,
            num_of_text_records: 1, // todo
            text_record_size: TEXT_RECORD_SIZE as u16,
            book_type: BookType::Book,
            text_encoding: Codepage::Utf8,
            uid: book.uid,
            file_version: 8,
            first_non_text_record: first_non_text_record as u32,
            language_code: LanguageCode {
                main: book.main_language.clone(),
                sub: book.sub_language.clone(),
            },
            first_resource_record: u32::MAX, // todo
            exth_flags: ExthFlags {
                has_exth: true,
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
            chunk_index: chunk_index_num as u32,
            skel_index: skeleton_index_num as u32,
            datp_index: u32::MAX,
            guide_index,
            exth: Some(exth),
        };
        let mut header_serialized = Cursor::new(vec![]);
        mobi_header.write(&mut header_serialized).unwrap();
        records[0] = header_serialized.into_inner();

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

    // todo: enable
    // proptest! {
    //     #[test]
    //     fn test_book_roundtrip(book in any::<Book>()) {
    //         env_logger::try_init();

    //         let mut serialized = Cursor::new(Vec::new());
    //         let mut writer = Writer::new(&mut serialized);
    //         book.to_writer(&mut writer, ()).unwrap();
    //         writer.finalize().unwrap();

    //         serialized.set_position(0);

    //         let mut reader = Reader::new(&mut serialized);
    //         let parsed = Book::from_reader_with_ctx(&mut reader, ()).unwrap();

    //         assert_eq!(book, parsed);
    //     }
    // }
}
