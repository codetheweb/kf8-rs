use crate::constants::{MetadataId, MetadataIdValue};
use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
};

use cookie_factory::gen;
use deku::{reader::Reader, writer::Writer, DekuError, DekuReader, DekuWriter};
#[cfg(test)]
use proptest_derive::Arbitrary;

mod read;
mod write;

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Exth {
    #[cfg_attr(test, proptest(filter = "|m| m.values().all(|v| v.len() > 0)"))]
    pub metadata_id: HashMap<MetadataId, Vec<String>>,
    #[cfg_attr(test, proptest(filter = "|m| m.values().all(|v| v.len() > 0)"))]
    pub metadata_value: HashMap<MetadataIdValue, Vec<u32>>,
}

impl Default for Exth {
    fn default() -> Self {
        Exth {
            metadata_id: HashMap::new(),
            metadata_value: HashMap::new(),
        }
    }
}

impl<'a, Ctx> DekuReader<'a, Ctx> for Exth {
    fn from_reader_with_ctx<R: Read>(reader: &mut Reader<R>, ctx: Ctx) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let mut tag = [0; 4];
        reader.read_bytes_const(&mut tag)?;
        if &tag != b"EXTH" {
            return Err(DekuError::Parse(
                format!("Expected EXTH tag, got {:?}", tag).into(),
            ));
        }

        let len = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Big)?;

        let mut buf = vec![0; len as usize];
        reader.read_bytes(len as usize, &mut buf)?;

        let (_, (metadata_id, metadata_value)) = read::read_exth(&buf).unwrap();

        Ok(Exth {
            metadata_id,
            metadata_value,
        })
    }
}

impl<Ctx> DekuWriter<Ctx> for Exth {
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _ctx: Ctx) -> Result<(), DekuError> {
        let serialized = Vec::new();
        let (serialized, _) = gen(write::write_exth(&self), serialized)
            .map_err(|_| DekuError::Io(ErrorKind::Other))?;

        writer.write_bytes(b"EXTH")?;

        let len = serialized.len() as u32;
        len.to_writer(writer, deku::ctx::Endian::Big)?;

        writer.write_bytes(&serialized)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::{arbitrary::any, proptest};

    proptest! {
      #[test]
      fn test_exth_roundtrip(exth in any::<Exth>()) {
        let mut serialized = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut serialized);
        exth.to_writer(&mut writer, ()).unwrap();
        writer.finalize().unwrap();

        serialized.set_position(0);
        let mut reader = Reader::new(&mut serialized);
        let decoded = Exth::from_reader_with_ctx(&mut reader, ()).unwrap();

        assert_eq!(exth, decoded);
      }
    }
}
