use deku::{writer::Writer, DekuWriter};
use kf8::serialization::{book::Book, BookPart};
use rand::Rng;

fn main() {
    let mut rng = rand::thread_rng();

    // let uid: u32 = rng.gen();
    let uid: u32 = 0x9CDB8CF6;

    let skeleton_head = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en-US">
  <head>
    <title>Titlepage</title>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
  <link rel="stylesheet" type="text/css" href="kindle:flow:0001?mime=text/css"/>
<link rel="stylesheet" type="text/css" href="kindle:flow:0002?mime=text/css"/>
</head>
  <body class="calibre" aid="0">
"#;
    let skeleton_tail = r#"</body>
</html>
"#;
    let slice = r#"<section class="epub-type-titlepage" id="titlepage" aid="1">
<h1 class="calibre1" aid="2">War and Peace</h1>
<p class="calibre2" aid="3">By <b class="calibre3" aid="4">Leo Tol\xc2\xadstoy</b>.</p>
<p class="calibre2" aid="5">Trans\xc2\xadlat\xc2\xaded by <b class="calibre3" aid="6">Louise Maude</b> and <b class="calibre3" aid="7">Aylmer Maude</b>.</p>
<img alt="" class="calibre4" src="kindle:embed:0004?mime=image/png"/>
</section>
"#;

    let book = Book {
        title: "Sample_.epub_Book".to_string(),
        uid,
        main_language: None,
        sub_language: None,
        book_parts: vec![BookPart {
            skeleton_head: skeleton_head.to_string(),
            content: slice.to_string(),
            skeleton_tail: skeleton_tail.to_string(),
        }],
    };

    let output = std::fs::File::create("hello_world.azw3").unwrap();
    let mut writer = Writer::new(output);
    book.to_writer(&mut writer, ()).unwrap();
}
