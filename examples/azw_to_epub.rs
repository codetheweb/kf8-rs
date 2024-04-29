use chrono::DateTime;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, Result, ZipLibrary};
use kf8::constants::MetadataId;
use kf8::{parse_book, MobiBook, ResourceKind};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use regex::{Regex, RegexBuilder};
use std::io::{Cursor, Read};
use std::iter::once;

#[macro_use]
extern crate lazy_static;

use clap::Parser;

/// Simple example of conversion from .azw3 to .epub.
/// For a more robust implementation, check out https://github.com/codetheweb/ignite
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file
    #[arg(short, long)]
    input: String,

    /// Output path
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();

    process(args).unwrap();
}

lazy_static! {
    static ref FLOW_PATTERN: Regex =
        RegexBuilder::new(r#"kindle:flow:([0-9|A-V]+)\?mime=([^'"]+)"#)
            .case_insensitive(true)
            .build()
            .unwrap();
}

fn transform_element(element: &mut BytesStart, book: &MobiBook) {
    let cloned = element.clone();
    let attributes = cloned.attributes();
    element.clear_attributes();

    for attribute in attributes.clone() {
        let attribute = attribute.as_ref().unwrap();

        match attribute.key {
            // Remap aid="" to id=""
            QName(b"aid") => {
                if attributes
                    .clone()
                    .any(|a| a.as_ref().unwrap().key == QName(b"id"))
                {
                    continue;
                }

                element.push_attribute(Attribute::from((
                    "id".as_bytes(),
                    [b"aid-", &attribute.value[..]].concat().as_slice(),
                )));
            }
            // Map flow links (kindle:flow:...) to local resources
            QName(b"href") if element.name() == QName(b"link") => {
                let value = attribute.value.to_vec();
                let value = String::from_utf8(value).unwrap();
                let captures = FLOW_PATTERN.captures(&value).unwrap();
                let flow_index = usize::from_str_radix(&captures[1], 10).unwrap();
                let mime_type = &captures[2];

                let href = match mime_type {
                    "text/css" => {
                        format!("styles_{}.css", flow_index)
                    }
                    _ => {
                        panic!("Unsupported flow type {}", mime_type)
                    }
                };

                element.push_attribute(Attribute::from(("href".as_bytes(), href.as_bytes())));
            }
            QName(b"href") if attribute.value.starts_with(b"kindle:pos:fid") => {
                element.push_attribute(Attribute::from((
                    "href".as_bytes(),
                    "https://example.com/".as_bytes(),
                    // book.get_id_for_position_href(&attribute.value)
                    //     .unwrap()
                    //     .as_bytes(),
                )));
            }
            _ => {
                element.push_attribute(attribute.clone());
            }
        }
    }
}

fn process(args: Args) -> Result<()> {
    let mut reader = std::fs::File::open(args.input).unwrap();
    let mut data = Vec::new();
    reader.read_to_end(&mut data).unwrap();

    let (_, book) = parse_book(&data).unwrap();

    let mut builder = EpubBuilder::new(ZipLibrary::new()?)?;
    builder.epub_version(EpubVersion::V30);

    // Text
    for part in &book.parts {
        let content_raw = once(part.skeleton_head.clone())
            .chain(part.fragments.iter().map(|f| f.content.clone()))
            .chain(once(part.skeleton_tail.clone()))
            .collect::<Vec<Vec<u8>>>()
            .concat();
        let content = String::from_utf8(content_raw).unwrap();

        let mut reader = quick_xml::reader::Reader::from_str(&content);
        let mut writer = quick_xml::writer::Writer::new(Cursor::new(Vec::new()));

        loop {
            match reader.read_event() {
                Ok(Event::Eof) => {
                    break;
                }
                Ok(Event::Start(mut element)) => {
                    transform_element(&mut element, &book);

                    writer.write_event(&Event::Start(element)).unwrap();
                }
                Ok(Event::Empty(mut element)) => {
                    transform_element(&mut element, &book);

                    writer.write_event(&Event::Empty(element)).unwrap();
                }
                Ok(event) => {
                    writer.write_event(&event).unwrap();
                }
                Err(e) => {
                    panic!("Error at position {}: {:?}", reader.buffer_position(), e);
                }
            }
        }

        let mut cursor = writer.into_inner();
        cursor.set_position(0);
        builder.add_content(EpubContent::new(part.filename.clone(), cursor))?;
    }

    builder.set_title(&book.book_header.title);

    // Metadata
    if let Some(ref metadata) = book.book_header.standard_metadata {
        if let Some(creators) = metadata.get(&MetadataId::Creator) {
            for creator in creators {
                builder.add_author(creator);
            }
        }

        if let Some(subjects) = metadata.get(&MetadataId::Subject) {
            builder.set_subjects(subjects.clone());
        }

        if let Some(descriptions) = metadata.get(&MetadataId::Description) {
            for description in descriptions {
                builder.add_description(description);
            }
        }

        if let Some(published) = metadata.get(&MetadataId::Published) {
            builder.set_publication_date(
                DateTime::parse_from_rfc3339(published.first().unwrap())
                    .unwrap()
                    .into(),
            );
        }

        if let Some(contributors) = metadata.get(&MetadataId::Contributor) {
            // todo: currently unsupported by epub library
        }

        if let Some(source) = metadata.get(&MetadataId::Source) {
            // todo: currently unsupported by epub library
        }

        if let Some(publishers) = metadata.get(&MetadataId::Publisher) {
            // todo: currently unsupported by epub library
        }
    }

    if let Some(language_tag) = book.book_header.get_bcp47_language_tag() {
        builder.set_lang(language_tag);
    }

    // todo: set the ID using the book_header.unique_id (unsupported by epub library?)

    // Resources
    for resource in &book.resources {
        match resource.kind {
            ResourceKind::Cover => {
                builder.add_cover_image(
                    format!("cover.{}", resource.file_type.extension()),
                    Cursor::new(resource.data.clone()),
                    resource.file_type.mime_type(),
                )?;
            }
            ResourceKind::Thumbnail => {
                // Don't output thumbnails
                continue;
            }
            ResourceKind::Image => {
                builder.add_resource(
                    "todo",
                    Cursor::new(resource.data.clone()),
                    resource.file_type.mime_type(),
                )?;
            }
            ResourceKind::Font => {
                todo!()
            }
            ResourceKind::Stylesheet => {
                builder.add_resource(
                    format!("styles_{}.css", resource.flow_index.unwrap_or_default()),
                    Cursor::new(resource.data.clone()),
                    resource.file_type.mime_type(),
                )?;
            }
        }
    }

    let writer = std::fs::File::create(args.output).unwrap();
    builder.generate(writer)?;

    Ok(())
}
