use deku::{writer::Writer, DekuWriter};
use kf8::serialization::book::Book;

fn main() {
    let book = Book {
        title: "Hello world".to_string(),
        uid: 1234,
        main_language: None,
        sub_language: None,
        text: "Hello world".to_string(),
    };

    let output = std::fs::File::create("hello_world.azw3").unwrap();
    let mut writer = Writer::new(output);
    book.to_writer(&mut writer, ()).unwrap();
}
