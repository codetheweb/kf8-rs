use epub_builder::{EpubBuilder, EpubContent, Result, ZipLibrary};
use kf8::constants::MetadataId;
use kf8::parse_book;
use std::io::Read;

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

    for part in book.parts {
        builder.add_content(EpubContent::new(part.filename, part.content.as_bytes()))?;
    }

    builder.set_title(book.book_header.title);

    if let Some(metadata) = book.book_header.exth {
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
    }

    let writer = std::fs::File::create(args.output).unwrap();
    builder.generate(writer)?;

    Ok(())
}
