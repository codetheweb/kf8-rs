use epub_builder::{EpubBuilder, Result, ZipLibrary};
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

    let writer = std::fs::File::create(args.output).unwrap();
    builder.generate(writer)?;

    Ok(())
}
