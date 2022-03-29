use crate::util::escape_and_elide;
use crate::{BodyAsyncReader, BodyReader};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::io::ErrorKind;
use std::path::PathBuf;
use temp_file::TempFile;

#[derive(Clone, Eq, PartialEq)]
pub enum ResponseBody {
    StaticStr(&'static str),
    Vec(Vec<u8>),
    File(PathBuf, u64),
    TempFile(TempFile, u64),
}
impl ResponseBody {
    #[must_use]
    pub fn empty() -> Self {
        ResponseBody::StaticStr("")
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn len(&self) -> u64 {
        match self {
            ResponseBody::StaticStr(s) => u64::try_from(s.len()).unwrap(),
            ResponseBody::Vec(v) => u64::try_from(v.len()).unwrap(),
            ResponseBody::File(.., len) | ResponseBody::TempFile(.., len) => *len,
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub fn reader(&self) -> Result<BodyReader<'_>, std::io::Error> {
        match self {
            ResponseBody::StaticStr(s) => Ok(BodyReader::bytes(s.as_bytes())),
            ResponseBody::Vec(v) => Ok(BodyReader::bytes(v.as_slice())),
            ResponseBody::File(path, ..) => BodyReader::file(path),
            ResponseBody::TempFile(temp_file, ..) => BodyReader::file(temp_file.path()),
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub async fn async_reader(&self) -> Result<BodyAsyncReader<'_>, std::io::Error> {
        match self {
            ResponseBody::StaticStr(s) => Ok(BodyAsyncReader::bytes(s.as_bytes())),
            ResponseBody::Vec(v) => Ok(BodyAsyncReader::bytes(v.as_slice())),
            ResponseBody::File(path, ..) => BodyAsyncReader::file(path).await,
            ResponseBody::TempFile(temp_file, ..) => BodyAsyncReader::file(temp_file.path()).await,
        }
    }
}
impl Debug for ResponseBody {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            ResponseBody::StaticStr(s) => write!(f, "ResponseBody::Str({:?})", s),
            ResponseBody::Vec(v) => write!(
                f,
                "ResponseBody::Vec({} {:?})",
                v.len(),
                escape_and_elide(v.as_slice(), 100)
            ),
            ResponseBody::File(path, len) => {
                write!(
                    f,
                    "ResponseBody::File({:?},{})",
                    path.to_string_lossy(),
                    len
                )
            }
            ResponseBody::TempFile(temp_file, len) => write!(
                f,
                "ResponseBody::TempFile({:?},{})",
                temp_file.path().to_string_lossy(),
                len
            ),
        }
    }
}

impl From<&'static str> for ResponseBody {
    fn from(s: &'static str) -> Self {
        ResponseBody::StaticStr(s)
    }
}
impl From<String> for ResponseBody {
    fn from(s: String) -> Self {
        ResponseBody::Vec(s.into_bytes())
    }
}
impl From<Vec<u8>> for ResponseBody {
    fn from(v: Vec<u8>) -> Self {
        ResponseBody::Vec(v)
    }
}
impl<const LEN: usize> From<[u8; LEN]> for ResponseBody {
    fn from(b: [u8; LEN]) -> Self {
        ResponseBody::Vec(b.to_vec())
    }
}
impl From<&[u8]> for ResponseBody {
    fn from(b: &[u8]) -> Self {
        ResponseBody::Vec(b.to_vec())
    }
}

impl TryFrom<ResponseBody> for String {
    type Error = std::io::Error;

    fn try_from(body: ResponseBody) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = body.try_into()?;
        String::from_utf8(bytes)
            .map_err(|_e| std::io::Error::new(ErrorKind::InvalidData, "message body is not UTF-8"))
    }
}
impl TryFrom<ResponseBody> for Vec<u8> {
    type Error = std::io::Error;

    fn try_from(body: ResponseBody) -> Result<Self, Self::Error> {
        match body {
            ResponseBody::StaticStr(s) => Ok(s.as_bytes().to_vec()),
            ResponseBody::Vec(v) => Ok(v),
            ResponseBody::File(path, ..) => std::fs::read(path),
            ResponseBody::TempFile(temp_file, ..) => std::fs::read(temp_file.path()),
        }
    }
}
