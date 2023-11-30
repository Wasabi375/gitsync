use std::{
    fs::OpenOptions,
    io::{empty, BufRead, BufReader, ErrorKind, Read, Write},
};

use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::Repository;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ObjectType {
    Blob,
    Commit,
    Tree,
    Tag,
}

#[derive(Debug)]
pub enum Object {
    Blob { data: Vec<u8> },
    Commit,
    Tree,
    Tag,
}

impl Object {
    pub fn serialize_zlib(&self) -> (String, Vec<u8>) {
        self.serialize_zlib_comp(Compression::default())
    }

    pub fn serialize_zlib_comp(&self, comp: Compression) -> (String, Vec<u8>) {
        let mut data = Vec::new();
        let mut encoder = ZlibEncoder::new(&mut data, comp);
        let hash = self.serialize_with_header(&mut encoder).unwrap();
        drop(encoder);
        (hash, data)
    }

    fn serialize_with_header(&self, write: &mut impl Write) -> Result<String> {
        let mut hasher = Sha1::new();
        let mut write = SplitWrite(write, &mut hasher);

        write!(write, "{} ", self.type_str())?;

        match self {
            Object::Blob { data } => {
                write!(write, "{}\0", data.len())?;
                write.write_all(&data)?;
            }
            _ => {
                let mut data = Vec::new();
                self.serialize(&mut data)?;
                write!(write, "{}\0", data.len())?;
                write.write_all(&data)?;
            }
        };
        drop(write);

        let mut hash = Vec::with_capacity(20);
        for c in hasher.finalize() {
            let _ = write!(hash, "{:0>2x}", c);
        }
        Ok(String::from_utf8(hash).unwrap())
    }

    pub fn serialize(&self, write: &mut impl Write) -> Result<()> {
        match self {
            Object::Blob { data } => write.write_all(&data)?,
            Object::Commit => todo!(),
            Object::Tree => todo!(),
            Object::Tag => todo!(),
        }
        Ok(())
    }

    pub fn save(&self, repo: &Repository) -> Result<String> {
        let (sha1, data) = self.serialize_zlib();

        let path = Repository::sha1_to_object(&sha1);
        let mut file = repo
            .file(path, OpenOptions::new().create(true).write(true), true)
            .context("save object")?;

        file.write_all(&data).context("save object")?;
        Ok(sha1)
    }

    pub fn sha1(&self) -> String {
        self.serialize_with_header(&mut empty()).unwrap()
    }

    fn type_str(&self) -> &'static str {
        match self {
            Object::Blob { data: _ } => "blob",
            Object::Commit => "commit",
            Object::Tree => "tree",
            Object::Tag => "tag",
        }
    }

    pub fn deserialize_zlib_read(reader: impl Read) -> Result<Self> {
        let bufferd = BufReader::new(reader);
        Self::deserialize_zlib(bufferd)
    }

    pub fn deserialize_zlib(data: impl BufRead) -> Result<Self> {
        let decoder = ZlibDecoder::new(data);
        let mut decoder = BufReader::new(decoder);

        let mut buf = Vec::new();
        decoder
            .read_until(b' ', &mut buf)
            .context("failed to read type")?;
        if buf.pop() != Some(b' ') {
            bail!("Expected b' ' after object type but got EOF instead");
        }
        let typ = String::from_utf8(buf).context(format!("Could not parse type"))?;
        let obj_type = match ObjectType::from_str(&typ, true) {
            Ok(typ) => typ,
            Err(msg) => bail!("Invalid blob type:\n{msg}"),
        };

        let mut buf = Vec::new();
        decoder
            .read_until(0, &mut buf)
            .context("failed to read type")?;
        if buf.pop() != Some(0) {
            bail!("Expected 0 after object size but got EOF instead");
        }
        let size = String::from_utf8(buf).context(format!("Could not parse size"))?;
        let size: usize = size.parse().context("could not parse size")?;

        let mut data = Vec::new();
        let real_size = decoder
            .read_to_end(&mut data)
            .context("could not read data")?;
        if real_size != size {
            bail!("Expected to read object of size {size} but got {real_size} instead");
        }

        Self::deserialize(obj_type, data)
    }

    pub fn deserialize_read(typ: ObjectType, reader: &mut impl Read) -> Result<Object> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        Self::deserialize(typ, data)
    }

    pub fn deserialize(typ: ObjectType, data: Vec<u8>) -> Result<Object> {
        match typ {
            ObjectType::Blob => Ok(Self::Blob { data }),
            ObjectType::Commit => todo!(),
            ObjectType::Tree => todo!(),
            ObjectType::Tag => todo!(),
        }
    }
}

struct SplitWrite<'l, A, B>(&'l mut A, &'l mut B);

impl<'l, A: Write, B: Write> Write for SplitWrite<'l, A, B> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let size_a = self.0.write(buf)?;
        let size_b = self.1.write(buf)?;

        if size_a != size_b {
            let err = std::io::Error::new(ErrorKind::Other, "write did not match in size");
            return Err(err);
        }
        Ok(size_a)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()?;
        self.1.flush()?;
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.0.write_all(buf)?;
        self.1.write_all(buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{fs::OpenOptions, io::Read};

    use flate2::{bufread::ZlibDecoder, Compression};
    use test_dir::DirBuilder;

    use crate::{test_utils, Object, ObjectType, Repository};

    #[test]
    fn read_blob_object() {
        const BLOB_SHA1: &str = "2bb09523ce4baf1940ee8fef49f6cade5afe3d03";

        let test_dir = test_utils::existing_test_repo("simple_test_blob");

        let repo = Repository::new(test_dir.root()).unwrap();

        let path = Repository::sha1_to_object(BLOB_SHA1);

        let obj_file = repo
            .file(path, OpenOptions::new().read(true), false)
            .unwrap();

        let obj = Object::deserialize_zlib_read(obj_file).unwrap();

        match obj {
            Object::Blob { data } => assert_eq!(data.as_slice(), b"this is a simple test blob\n"),
            _ => panic!("expected blob!"),
        }
    }

    #[test]
    fn blob_sha1() {
        const BLOB_DATA: &[u8] = b"this is a simple test blob\n";
        const BLOB_SHA1: &str = "2bb09523ce4baf1940ee8fef49f6cade5afe3d03";

        let obj = Object::deserialize(ObjectType::Blob, BLOB_DATA.into()).unwrap();

        assert_eq!(obj.sha1(), BLOB_SHA1);
    }

    #[test]
    #[ignore = "cant reproduce gits zlib compression."]
    fn zlib_simple_blob() {
        const BLOB_SHA1: &str = "2bb09523ce4baf1940ee8fef49f6cade5afe3d03";
        const BLOB_DATA: &[u8] = b"this is a simple Test blob\n";

        let test_dir = test_utils::existing_test_repo("simple_test_blob");

        let repo = Repository::new(test_dir.root()).unwrap();

        let path = Repository::sha1_to_object(BLOB_SHA1);

        let mut obj_file = repo
            .file(path, OpenOptions::new().read(true), false)
            .unwrap();
        let mut expected = Vec::new();
        obj_file.read_to_end(&mut expected).unwrap();

        let obj = Object::deserialize(ObjectType::Blob, BLOB_DATA.into()).unwrap();

        let (sha1, result) = obj.serialize_zlib_comp(Compression::new(1));

        assert_eq!(sha1, BLOB_SHA1);
        assert_eq!(result, expected);
    }

    #[test]
    fn roundtip_no_zlib() {
        const BLOB_SHA1: &str = "2bb09523ce4baf1940ee8fef49f6cade5afe3d03";

        let test_dir = test_utils::existing_test_repo("simple_test_blob");

        let repo = Repository::new(test_dir.root()).unwrap();

        let path = Repository::sha1_to_object(BLOB_SHA1);

        let mut obj_file = repo
            .file(path, OpenOptions::new().read(true), false)
            .unwrap();

        let mut zlib = Vec::new();
        obj_file.read_to_end(&mut zlib).unwrap();

        let obj = Object::deserialize_zlib(zlib.as_slice()).unwrap();

        let mut result = Vec::new();
        let sha1 = obj.serialize_with_header(&mut result).unwrap();
        assert_eq!(sha1, BLOB_SHA1);

        let mut expected = Vec::new();
        let mut decoder = ZlibDecoder::new(zlib.as_slice());
        decoder.read_to_end(&mut expected).unwrap();

        assert_eq!(result, expected);
    }
}
