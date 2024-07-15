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

    if buf[0] == 0 {
        return Ok("".to_string());
    }

    let first_null = buf.iter().position(|&x| x == 0).unwrap_or(len);

    let str = String::from_utf8(buf[..first_null].to_vec())
        .map_err(|_| DekuError::Parse("Invalid UTF-8".into()))?;

    Ok(str)
}

pub(crate) fn write_fixed_length_string<W: Write>(
    writer: &mut deku::writer::Writer<W>,
    s: &str,
    len: usize,
) -> Result<(), DekuError> {
    let mut buf = s.as_bytes().to_vec();
    buf.resize(len, 0);
    writer.write_bytes(&buf)
}
