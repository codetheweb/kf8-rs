use deku::{writer::Writer, DekuWriter};
use kf8::serialization::book::Book;

fn main() {
    let book = Book {
        title: "Hello world".to_string(),
        uid: 1234,
        main_language: None,
        sub_language: None,
        text: r#"
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en-US">
  <head>
    <title>Titlepage</title>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
</head>
  <body>
		<section class="epub-type-titlepage" id="titlepage">
			<h1 class="calibre1">War and Peace</h1>
			<p class="calibre2">By <b class="calibre3">Leo Tol­stoy</b>.</p>
			<p class="calibre2">Trans­lat­ed by <b class="calibre3">Louise Maude</b> and <b class="calibre3">Aylmer Maude</b>.</p>
		</section>
	</body>
</html>
"#.to_string(),
    };

    let output = std::fs::File::create("hello_world.azw3").unwrap();
    let mut writer = Writer::new(output);
    book.to_writer(&mut writer, ()).unwrap();
}
