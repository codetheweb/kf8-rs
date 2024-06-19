use chrono::DateTime;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, Result, ZipLibrary};
use kf8::constants::MetadataId;
use kf8::{parse_book, ImageResourceKind, MobiBook, ResourceKind};
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

lazy_static! {
    static ref POSITION_FID_PATTERN: Regex =
        RegexBuilder::new(r#"kindle:pos:fid:([0-9|A-V]+):off:([0-9|A-V]+)"#)
            .case_insensitive(true)
            .build()
            .unwrap();
}

lazy_static! {
    static ref ID_OR_NAME_OR_AID_PATTERN: Regex =
        RegexBuilder::new(r#"\s(id|name|aid)\s*=\s*['"]([^'"]*)['"]"#)
            .case_insensitive(true)
            .build()
            .unwrap();
}

lazy_static! {
    static ref EMBED_PATTERN: Regex =
        RegexBuilder::new(r#"kindle:embed:([0-9|A-V]+)\?mime=([^'"]+)"#)
            .case_insensitive(true)
            .build()
            .unwrap();
}

fn transform_element(element: &mut BytesStart, image_paths: &Vec<String>, book: &MobiBook) {
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
            // Remap kindle:pos:fid:... to filenames and anchors (href="filename#anchor")
            QName(b"href") if attribute.value.starts_with(b"kindle:pos:fid") => {
                let value = attribute.value.to_vec();
                let value = String::from_utf8(value).unwrap();
                let captures = POSITION_FID_PATTERN.captures(&value).unwrap();
                let absolute_fragment_index = usize::from_str_radix(&captures[1], 32).unwrap();
                let offset = usize::from_str_radix(&captures[2], 32).unwrap();

                let fragment = book.fragment_table.get(absolute_fragment_index).unwrap();
                let position = fragment.insert_position as usize + offset;

                let part = book
                    .parts
                    .iter()
                    .find(|part| position >= part.start_offset && position < part.end_offset)
                    .unwrap();

                let offset = position - part.start_offset;
                let content = part.get_content();
                let selected_content = String::from_utf8_lossy(&content[offset..]);

                let captures = ID_OR_NAME_OR_AID_PATTERN
                    .captures(&selected_content)
                    .unwrap();
                let attribute_value = captures.get(2).unwrap().as_str();

                element.push_attribute(Attribute::from((
                    "href".as_bytes(),
                    format!("{}#{}", part.filename, attribute_value).as_bytes(),
                )));
            }
            // Map kindle:embed:... to local resources (for images)
            QName(b"src") if element.name() == QName(b"img") => {
                let value = attribute.value.to_vec();
                let value = String::from_utf8(value).unwrap();
                let captures = EMBED_PATTERN.captures(&value);

                // todo: should log a warning here?
                if let Some(captures) = captures {
                    let image_index = usize::from_str_radix(&captures[1], 32).unwrap() - 1;

                    element.push_attribute(Attribute::from((
                        "src".as_bytes(),
                        image_paths[image_index].as_bytes(),
                    )));
                }
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

    // Resources
    let mut image_paths = Vec::new();
    // todo: cleaner
    let mut image_i = 0;
    for resource in &book.resources {
        match resource.kind {
            ResourceKind::Image(ImageResourceKind::Cover) => {
                let path = format!("cover.{}", resource.file_type.extension());
                image_paths.push(path.clone());

                builder.add_cover_image(
                    path,
                    Cursor::new(resource.data.clone()),
                    resource.file_type.mime_type(),
                )?;
            }
            // todo: handle thumbnail separately?
            ResourceKind::Image(..) => {
                let path = format!("images/{}.{}", image_i, resource.file_type.extension());
                image_paths.push(path.clone());

                builder.add_resource(
                    path,
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

        match resource.kind {
            ResourceKind::Image(..) => {
                image_i += 1;
            }
            _ => (),
        }
    }

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
                    transform_element(&mut element, &image_paths, &book);

                    writer.write_event(&Event::Start(element)).unwrap();
                }
                Ok(Event::Empty(mut element)) => {
                    transform_element(&mut element, &image_paths, &book);

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

    let writer = std::fs::File::create(args.output).unwrap();
    builder.generate(writer)?;

    Ok(())
}
