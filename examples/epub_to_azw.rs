use std::{
    borrow::Borrow,
    io::{Cursor, Write},
};

use binrw::BinRead;
use deku::{writer::Writer, DekuContainerRead, DekuWriter};
use kf8::{
    constants::MainLanguage,
    serialization::{book::Book, BookPart, CompressionType, MobiHeader, PalmDoc},
};
use rand::Rng;

const CSS_CONTENT: &str = r#"
.calibre {
    display: block;
    font-size: 1em;
    padding-left: 0;
    padding-right: 0;
    margin: 0 5pt
    }
.calibre1 {
    display: block;
    font-size: 2em;
    font-weight: bold;
    line-height: 1.2;
    margin: 0.67em 0
    }
.calibre2 {
    display: block;
    margin: 1em 0
    }
.calibre3 {
    font-weight: bold
    }
"#;

fn main() {
    let mut rng = rand::thread_rng();

    // let uid: u32 = rng.gen();
    let uid: u32 = 0x9CDB8CF6;

    let skeleton_head = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en-US">
  <head>
    <title>Titlepage</title>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
</head>
  <body class="calibre" aid="0">
"#;
    let skeleton_tail = r#"</body>
</html>
"#;
    let slice = r#"<section class="epub-type-titlepage" id="titlepage" aid="1">
<h1 class="calibre1" aid="2">War and Peace</h1>
<p class="calibre2" aid="3">By <b class="calibre3" aid="4">Leo</b>.</p>
<p class="calibre2" aid="5">Trans by <b class="calibre3" aid="6">Louise Maude</b> and <b class="calibre3" aid="7">Aylmer Maude</b>.</p>
</section>
"#;

    let book = Book {
        title: "Sample_.epub_Book".to_string(),
        uid,
        main_language: Some(MainLanguage::English),
        sub_language: None,
        book_parts: vec![BookPart {
            skeleton_head: skeleton_head.to_string(),
            content: slice.to_string(),
            skeleton_tail: skeleton_tail.to_string(),
        }],
        resources: vec![CSS_CONTENT.to_string()],
        compression: CompressionType::None,
    };

    let mut output = std::fs::File::create("hello_world.azw3").unwrap();
    let mut writer = Writer::new(&mut output);
    book.to_writer(&mut writer, ()).unwrap();
    writer.finalize().unwrap();
    output.flush().unwrap();

    // let mut input = std::fs::File::open("minimal-calibre.azw3").unwrap();
    // let palmdoc = PalmDoc::from_reader((&mut input, 0)).unwrap();
    // let mut first_record = Cursor::new(palmdoc.1.records[0].clone());
    // let header = MobiHeader::read(&mut first_record).unwrap();
    // println!("{:?}", header);
}
