use std::io::Write;

use cookie_factory::SerializeFn;
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
    if len == 0 {
        return Ok("".to_string());
    }

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

pub(crate) fn read_big_endian_variable_width_value<R: std::io::Read>(
    reader: &mut deku::reader::Reader<R>,
) -> Result<u32, DekuError> {
    let mut bytes = Vec::new();
    let mut buf = [0; 1];
    loop {
        reader.read_bytes(1, &mut buf)?;
        let byte = buf[0];
        bytes.push(byte & 0b01111111);
        if byte & 0b10000000 != 0 {
            break;
        }
    }

    let mut value = 0;
    for byte in bytes {
        value <<= 7;
        value |= byte as u32;
    }

    Ok(value)
}

pub(crate) fn read_little_endian_variable_width_value<R: std::io::Read>(
    reader: &mut deku::reader::Reader<R>,
    len: usize,
) -> Result<u32, DekuError> {
    let mut bytes = Vec::new();
    let mut src = vec![0; len];
    reader.read_bytes(len, &mut src)?;
    src.reverse();

    for byte in src {
        bytes.push(byte & 0b01111111);
        if byte & 0b10000000 != 0 {
            break;
        }
    }

    bytes.reverse();

    let mut value = 0;
    for byte in bytes {
        value <<= 7;
        value |= byte as u32;
    }

    Ok(value)
}

pub(crate) fn serialize_variable_width_value(value: u32, endian: deku::ctx::Endian) -> Vec<u8> {
    let mut value = value;
    let mut buf = Vec::new();

    loop {
        let b = (value & 0b01111111) as u8;
        value >>= 7;
        buf.push(b);
        if value == 0 {
            break;
        }
    }

    if endian == deku::ctx::Endian::Big {
        buf[0] |= 0b10000000;
    } else {
        let len = buf.len();
        buf[len - 1] |= 0b10000000;
    }

    buf.reverse();

    buf
}

pub(crate) fn cookie_write_variable_width_value<W: Write>(
    value: u32,
    endian: deku::ctx::Endian,
) -> impl SerializeFn<W> {
    let buf = serialize_variable_width_value(value, endian);

    move |mut w| {
        w.write_all(&buf)?;
        Ok(w)
    }
}

pub(crate) fn write_variable_width_value<W: Write>(
    writer: &mut deku::writer::Writer<W>,
    value: u32,
    endian: deku::ctx::Endian,
) -> Result<(), DekuError> {
    let buf = serialize_variable_width_value(value, endian);
    writer.write_bytes(&buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use deku::prelude::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use std::io::Cursor;

    proptest! {
        #[test]
        fn test_variable_width_value_roundtrip(value in 0..u32::MAX, is_big_endian in any::<bool>()) {
            let endian = if is_big_endian {
                deku::ctx::Endian::Big
            } else {
                deku::ctx::Endian::Little
            };

            let mut serialized = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut serialized);
            write_variable_width_value(&mut writer, value, endian).unwrap();
            writer.finalize().unwrap();

            let len = serialized.get_ref().len();
            serialized.set_position(0);
            let mut reader = Reader::new(&mut serialized);
            let decoded = if is_big_endian {
                 read_big_endian_variable_width_value(&mut reader).unwrap()
            } else {
                    read_little_endian_variable_width_value(&mut reader, len).unwrap()
                };


            assert_eq!(value, decoded);
        }
    }
}
