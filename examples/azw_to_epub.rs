use chrono::DateTime;
use epub_builder::{EpubBuilder, EpubContent, Result, ZipLibrary};
use kf8::constants::MetadataId;
use kf8::{parse_book, ResourceKind};
use regex::{Captures, RegexBuilder};
use std::io::{Cursor, Read};

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

fn process(args: Args) -> Result<()> {
    let mut reader = std::fs::File::open(args.input).unwrap();
    let mut data = Vec::new();
    reader.read_to_end(&mut data).unwrap();

    let (_, book) = parse_book(&data).unwrap();

    let mut builder = EpubBuilder::new(ZipLibrary::new()?)?;

    let flow_pattern = RegexBuilder::new(r#"['"]kindle:flow:([0-9|A-V]+)\?mime=([^'"]+)['"]"#)
        .case_insensitive(true)
        .build()
        .unwrap();

    // Text
    for part in book.parts {
        let updated_content = flow_pattern
            .replace_all(&part.content, |captures: &Captures| {
                let flow_index = usize::from_str_radix(&captures[1], 10);
                let href = match &captures[2] {
                    "text/css" => {
                        format!("styles_{}.css", flow_index.unwrap_or_default())
                    }
                    _ => {
                        panic!("Unsupported flow type {}", &captures[1])
                    }
                };

                format!("\"{}\"", href)
            })
            .to_string();

        builder.add_content(EpubContent::new(part.filename, updated_content.as_bytes()))?;
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
