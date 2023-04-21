use crate::http_error::HttpError;
use crate::util::{copy_async, escape_and_elide, CopyResult};
use crate::{BodyAsyncReader, BodyReader};
use futures_io::AsyncRead;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use temp_file::TempFile;

#[must_use]
fn cannot_read_pending_body_error() -> std::io::Error {
    std::io::Error::new(
        ErrorKind::InvalidInput,
        "cannot read pending body; your handler did not return Response::get_body_and_reprocess()",
    )
}

#[derive(Clone, Eq, PartialEq)]
pub enum RequestBody {
    PendingKnown(u64),
    PendingUnknown,
    StaticBytes(&'static [u8]),
    StaticStr(&'static str),
    Vec(Vec<u8>),
    File(PathBuf, u64),
    TempFile(TempFile, u64),
}
impl RequestBody {
    #[must_use]
    pub fn empty() -> Self {
        RequestBody::StaticStr("")
    }

    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(
            self,
            RequestBody::PendingKnown(..) | RequestBody::PendingUnknown
        )
    }

    #[must_use]
    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    /// Returns the body length, if it is known.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::match_same_arms)]
    pub fn len(&self) -> Option<u64> {
        match self {
            RequestBody::PendingUnknown => None,
            RequestBody::PendingKnown(len) => Some(*len),
            RequestBody::StaticBytes(b) => Some(u64::try_from(b.len()).unwrap()),
            RequestBody::StaticStr(s) => Some(u64::try_from(s.len()).unwrap()),
            RequestBody::Vec(v) => Some(u64::try_from(v.len()).unwrap()),
            RequestBody::File(.., len) | RequestBody::TempFile(.., len) => Some(*len),
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub fn reader(&self) -> Result<BodyReader<'_>, std::io::Error> {
        match self {
            RequestBody::PendingKnown(..) | RequestBody::PendingUnknown => {
                Err(cannot_read_pending_body_error())
            }
            RequestBody::StaticBytes(b) => Ok(BodyReader::Cursor(Cursor::new(b))),
            RequestBody::StaticStr(s) => Ok(BodyReader::Cursor(Cursor::new(s.as_bytes()))),
            RequestBody::Vec(v) => Ok(BodyReader::Cursor(Cursor::new(v))),
            RequestBody::File(path, ..) => std::fs::File::open(path).map(BodyReader::File),
            RequestBody::TempFile(temp_file, ..) => {
                std::fs::File::open(temp_file.path()).map(BodyReader::File)
            }
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub async fn async_reader(&self) -> Result<BodyAsyncReader<'_>, std::io::Error> {
        match self {
            RequestBody::PendingKnown(..) | RequestBody::PendingUnknown => {
                Err(cannot_read_pending_body_error())
            }
            RequestBody::StaticBytes(b) => Ok(BodyAsyncReader::Cursor(Cursor::new(b))),
            RequestBody::StaticStr(s) => Ok(BodyAsyncReader::Cursor(Cursor::new(s.as_bytes()))),
            RequestBody::Vec(v) => Ok(BodyAsyncReader::Cursor(Cursor::new(v))),
            RequestBody::File(path, ..) => {
                Ok(BodyAsyncReader::File(async_fs::File::open(path).await?))
            }
            RequestBody::TempFile(temp_file, ..) => Ok(BodyAsyncReader::File(
                async_fs::File::open(temp_file.path()).await?,
            )),
        }
    }
}
impl Debug for RequestBody {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            RequestBody::PendingKnown(len) => {
                write!(f, "RequestBody::PendingKnown(len={len})")
            }
            RequestBody::PendingUnknown => write!(f, "RequestBody::PendingUnknown"),
            RequestBody::StaticBytes(b) => {
                write!(
                    f,
                    "RequestBody::StaticBytes(len={} [{}])",
                    b.len(),
                    escape_and_elide(b, 100),
                )
            }
            RequestBody::StaticStr(s) => write!(
                f,
                "RequestBody::StaticStr(Str(len={} \"{}\")",
                s.len(),
                escape_and_elide(s.as_bytes(), 100),
            ),
            RequestBody::Vec(v) => write!(
                f,
                "RequestBody::Vec(len={} [{}])",
                v.len(),
                escape_and_elide(v.as_slice(), 100)
            ),
            RequestBody::File(path, len) => {
                write!(
                    f,
                    "RequestBody::File(len={}, path={:?})",
                    len,
                    path.to_string_lossy()
                )
            }
            RequestBody::TempFile(temp_file, len) => write!(
                f,
                "RequestBody::TempFile(len={}, path={:?})",
                len,
                temp_file.path().to_string_lossy(),
            ),
        }
    }
}
impl From<&'static [u8]> for RequestBody {
    fn from(b: &'static [u8]) -> Self {
        RequestBody::StaticBytes(b)
    }
}
impl From<&'static str> for RequestBody {
    fn from(s: &'static str) -> Self {
        RequestBody::StaticStr(s)
    }
}
impl From<String> for RequestBody {
    fn from(s: String) -> Self {
        RequestBody::Vec(s.into_bytes())
    }
}
impl From<Vec<u8>> for RequestBody {
    fn from(v: Vec<u8>) -> Self {
        RequestBody::Vec(v)
    }
}
impl<const LEN: usize> From<[u8; LEN]> for RequestBody {
    fn from(b: [u8; LEN]) -> Self {
        RequestBody::Vec(b.to_vec())
    }
}
impl TryFrom<RequestBody> for String {
    type Error = std::io::Error;

    fn try_from(body: RequestBody) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = body.try_into()?;
        String::from_utf8(bytes)
            .map_err(|_e| std::io::Error::new(ErrorKind::InvalidData, "message body is not UTF-8"))
    }
}
impl TryFrom<RequestBody> for Vec<u8> {
    type Error = std::io::Error;

    fn try_from(body: RequestBody) -> Result<Self, Self::Error> {
        match body {
            RequestBody::PendingKnown(..) | RequestBody::PendingUnknown => {
                Err(cannot_read_pending_body_error())
            }
            RequestBody::StaticBytes(b) => Ok(b.to_vec()),
            RequestBody::StaticStr(s) => Ok(s.as_bytes().to_vec()),
            RequestBody::Vec(v) => Ok(v),
            RequestBody::File(path, ..) => std::fs::read(path),
            RequestBody::TempFile(temp_file, ..) => std::fs::read(temp_file.path()),
        }
    }
}

/// # Errors
/// Returns an error when we fail to read the entire request body from the connection
pub async fn read_http_body_to_vec(
    reader: impl AsyncRead + Unpin,
    len: usize,
) -> Result<RequestBody, HttpError> {
    //dbg!("read_http_body_to_vec", len);
    let mut body_vec = Vec::with_capacity(len);
    AsyncReadExt::take(reader, len as u64)
        .read_to_end(&mut body_vec)
        .await
        .map_err(|_e| HttpError::Truncated)?;
    if body_vec.len() < len {
        return Err(HttpError::Truncated);
    }
    Ok(RequestBody::Vec(body_vec))
}

/// # Errors
/// Returns an error when:
/// - the request body is longer than `max_len`
/// - we fail to read the request body from the connection
pub async fn read_http_unsized_body_to_vec(
    mut reader: impl AsyncRead + Unpin,
) -> Result<RequestBody, HttpError> {
    //dbg!("read_http_unsized_body_to_vec");
    let mut body_vec = Vec::new();
    reader
        .read_to_end(&mut body_vec)
        .await
        .map_err(|_| HttpError::Truncated)?;
    Ok(RequestBody::Vec(body_vec))
}

/// # Errors
/// Returns an error when:
/// - we fail to read the entire request body from the connection
/// - we fail to open a temporary file
/// - we fail to write the body to the file
pub async fn read_http_body_to_file(
    reader: impl AsyncRead + Unpin,
    len: u64,
    dir: &Path,
) -> Result<RequestBody, HttpError> {
    //dbg!("read_http_body_to_file", len, dir);
    // TODO: Add async support to `temp_file` and use it here.
    let temp_file = TempFile::in_dir(dir).map_err(HttpError::error_saving_file)?;
    let mut file = async_fs::File::create(temp_file.path())
        .await
        .map_err(HttpError::error_saving_file)?;
    match copy_async(&mut AsyncReadExt::take(reader, len), &mut file).await {
        CopyResult::Ok(num_copied) if num_copied == len => {}
        CopyResult::Ok(..) | CopyResult::ReaderErr(..) => return Err(HttpError::Truncated),
        CopyResult::WriterErr(e) => return Err(HttpError::error_saving_file(e)),
    }
    file.close().await.map_err(HttpError::error_saving_file)?;
    Ok(RequestBody::TempFile(temp_file, len))
}

/// # Errors
/// Returns an error when:
/// - the request body is longer than `max_len`
/// - we fail to read the request body from the connection
/// - we fail to open a temporary file
/// - we fail to write the body to the file
pub async fn read_http_unsized_body_to_file(
    reader: impl AsyncRead + Unpin,
    dir: &Path,
    max_len: u64,
) -> Result<RequestBody, HttpError> {
    //dbg!("read_http_body_to_file", max_len, dir);
    let temp_file = TempFile::in_dir(dir).map_err(HttpError::error_saving_file)?;
    let mut file = async_fs::File::create(temp_file.path())
        .await
        .map_err(HttpError::error_saving_file)?;
    let len = match copy_async(AsyncReadExt::take(reader, max_len + 1), &mut file).await {
        CopyResult::Ok(len) => len,
        CopyResult::ReaderErr(..) => return Err(HttpError::Truncated),
        CopyResult::WriterErr(e) => return Err(HttpError::error_saving_file(e)),
    };
    file.close().await.map_err(HttpError::error_saving_file)?;
    if max_len < len {
        return Err(HttpError::BodyTooLong);
    }
    Ok(RequestBody::TempFile(temp_file, len))
}
