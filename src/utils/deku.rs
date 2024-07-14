use std::io::Write;

use deku::DekuError;

pub(crate) fn write_string<W: Write>(
    writer: &mut deku::writer::Writer<W>,
    s: &str,
) -> Result<(), DekuError> {
    writer.write_bytes(s.as_bytes())
}

pub(crate) fn read_string<R: std::io::Read>(
    reader: &mut deku::reader::Reader<R>,
    len: usize,
) -> Result<String, DekuError> {
    let mut buf = vec![0; len];
    reader.read_bytes(len, &mut buf)?;
    String::from_utf8(buf).map_err(|_| DekuError::Parse("Invalid UTF-8".into()))
}
