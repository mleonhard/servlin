use crate::event::EventReceiver;
use crate::util::escape_and_elide;
use crate::{BodyAsyncReader, BodyReader};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::io::{ErrorKind, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use temp_file::TempFile;

pub enum ResponseBody {
    EventStream(Mutex<EventReceiver>),
    StaticBytes(&'static [u8]),
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
    #[allow(clippy::missing_panics_doc)]
    pub fn is_empty(&self) -> bool {
        #[allow(clippy::match_same_arms)]
        match self.len() {
            None => false,
            Some(0) => true,
            Some(_) => false,
        }
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn len(&self) -> Option<u64> {
        match self {
            ResponseBody::EventStream(..) => None,
            ResponseBody::StaticBytes(b) => Some(u64::try_from(b.len()).unwrap()),
            ResponseBody::StaticStr(s) => Some(u64::try_from(s.len()).unwrap()),
            ResponseBody::Vec(v) => Some(u64::try_from(v.len()).unwrap()),
            ResponseBody::File(.., len) | ResponseBody::TempFile(.., len) => Some(*len),
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub fn reader(&self) -> Result<BodyReader<'_>, std::io::Error> {
        match self {
            ResponseBody::EventStream(mutex_receiver) => {
                Ok(BodyReader::EventReceiver(mutex_receiver))
            }
            ResponseBody::StaticBytes(b) => Ok(BodyReader::bytes(b)),
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
            ResponseBody::EventStream(mutex_receiver) => {
                Ok(BodyAsyncReader::EventReceiver(mutex_receiver))
            }
            ResponseBody::StaticBytes(b) => Ok(BodyAsyncReader::bytes(b)),
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
            ResponseBody::EventStream(..) => write!(f, "ResponseBody::EventStream(..)"),
            ResponseBody::StaticBytes(b) => {
                write!(
                    f,
                    "ResponseBody::StaticBytes(len={} [{}])",
                    b.len(),
                    escape_and_elide(b, 100),
                )
            }
            ResponseBody::StaticStr(s) => write!(
                f,
                "ResponseBody::StaticStr(len={} \"{}\")",
                s.len(),
                escape_and_elide(s.as_bytes(), 100),
            ),
            ResponseBody::Vec(v) => write!(
                f,
                "ResponseBody::Vec(len={} [{}])",
                v.len(),
                escape_and_elide(v.as_slice(), 100)
            ),
            ResponseBody::File(path, len) => {
                write!(
                    f,
                    "ResponseBody::File(len={}, path={:?})",
                    len,
                    path.to_string_lossy()
                )
            }
            ResponseBody::TempFile(temp_file, len) => write!(
                f,
                "ResponseBody::TempFile(len={}, path={:?})",
                len,
                temp_file.path().to_string_lossy(),
            ),
        }
    }
}
impl Eq for ResponseBody {}
impl From<&'static [u8]> for ResponseBody {
    fn from(b: &'static [u8]) -> Self {
        ResponseBody::StaticBytes(b)
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
impl PartialEq for ResponseBody {
    fn eq(&self, other: &Self) -> bool {
        #[allow(clippy::match_same_arms)]
        match (self, other) {
            (ResponseBody::EventStream(..), ResponseBody::EventStream(..)) => false,
            (ResponseBody::StaticBytes(b1), ResponseBody::StaticBytes(b2)) => b1 == b2,
            (ResponseBody::StaticStr(s1), ResponseBody::StaticStr(s2)) => s1 == s2,
            (ResponseBody::Vec(v1), ResponseBody::Vec(v2)) => v1 == v2,
            (ResponseBody::File(path1, len1), ResponseBody::File(path2, len2)) => {
                path1 == path2 && len1 == len2
            }
            (
                ResponseBody::TempFile(temp_file1, len1),
                ResponseBody::TempFile(temp_file2, len2),
            ) => temp_file1.path() == temp_file2.path() && len1 == len2,
            _ => false,
        }
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
            ResponseBody::EventStream(mutex_receiver) => {
                let mut buf = Vec::new();
                mutex_receiver.lock().unwrap().read_to_end(&mut buf)?;
                Ok(buf)
            }
            ResponseBody::StaticBytes(b) => Ok(b.to_vec()),
            ResponseBody::StaticStr(s) => Ok(s.as_bytes().to_vec()),
            ResponseBody::Vec(v) => Ok(v),
            ResponseBody::File(path, ..) => std::fs::read(path),
            ResponseBody::TempFile(temp_file, ..) => std::fs::read(temp_file.path()),
        }
    }
}
