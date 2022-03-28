use crate::http_error::HttpError;
use crate::util::{copy_async, escape_and_elide, CopyResult};
use crate::Response;
use futures_io::AsyncRead;
use futures_lite::{AsyncReadExt, AsyncWriteExt, FutureExt};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::future::Future;
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use temp_file::TempFile;

#[must_use]
fn cannot_read_pending_body_error() -> std::io::Error {
    std::io::Error::new(ErrorKind::InvalidInput, "cannot read Body::Pending")
}

/// Future returned by `Body::async_reader`.
#[allow(clippy::module_name_repetitions)]
pub struct BodyAsyncReader<'x> {
    body: &'x Body,
    offset: usize,
    #[allow(clippy::type_complexity)]
    open_fut: Option<Pin<Box<dyn Future<Output = Result<async_fs::File, std::io::Error>> + Send>>>,
    file: Option<Pin<Box<async_fs::File>>>,
}
impl<'x> BodyAsyncReader<'x> {
    fn copy_bytes(&mut self, buf: &mut [u8], src: &[u8]) -> Poll<std::io::Result<usize>> {
        let available = &src[self.offset..];
        let len = available.len().min(buf.len());
        buf[..len].copy_from_slice(&available[..len]);
        self.offset += len;
        Poll::Ready(Ok(len))
    }

    #[allow(clippy::redundant_else)]
    #[allow(clippy::unnecessary_to_owned)]
    fn read_file(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        path: &Path,
    ) -> Poll<std::io::Result<usize>> {
        loop {
            if let Some(ref mut file) = &mut self.file {
                return Pin::new(file).poll_read(cx, buf);
            } else if let Some(open_fut) = &mut self.open_fut {
                match open_fut.poll(cx) {
                    Poll::Ready(Ok(file)) => {
                        self.open_fut.take();
                        self.file = Some(Box::pin(file));
                        continue;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            } else {
                self.open_fut = Some(Box::pin(async_fs::File::open(path.to_path_buf())));
                continue;
            }
        }
    }
}
impl<'x> futures_io::AsyncRead for BodyAsyncReader<'x> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        match &self.body {
            Body::Empty => Poll::Ready(Ok(0)),
            Body::Pending(..) => Poll::Ready(Err(cannot_read_pending_body_error())),
            Body::Str(s) => self.copy_bytes(buf, s.as_bytes()),
            Body::String(s) => self.copy_bytes(buf, s.as_bytes()),
            Body::Vec(b) => self.copy_bytes(buf, b),
            Body::File(path, ..) => self.read_file(cx, buf, path.as_path()),
            Body::TempFile(temp_file, ..) => self.read_file(cx, buf, temp_file.path()),
        }
    }
}

/// Struct returned by `Body::reader`.
#[allow(clippy::module_name_repetitions)]
pub enum BodyReader<'x> {
    Cursor(Cursor<&'x [u8]>),
    File(std::fs::File),
}
impl<'x> std::io::Read for BodyReader<'x> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BodyReader::Cursor(cursor) => cursor.read(buf),
            BodyReader::File(file) => file.read(buf),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Body {
    // TODO: Remove `Empty` and use `Body::Vec(Vec::new())` instead.
    Empty,
    Pending(Option<u64>),
    Str(&'static str),
    String(String),
    Vec(Vec<u8>),
    File(PathBuf, u64),
    TempFile(TempFile, u64),
}

impl Body {
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self, Body::Pending(..))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn len(&self) -> u64 {
        match self {
            Body::Empty | Body::Pending(None) => 0,
            Body::Pending(Some(n)) => *n,
            Body::Str(s) => u64::try_from(s.len()).unwrap(),
            Body::String(s) => u64::try_from(s.len()).unwrap(),
            Body::Vec(v) => u64::try_from(v.len()).unwrap(),
            Body::File(.., len) | Body::TempFile(.., len) => *len,
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    pub fn reader(&self) -> Result<BodyReader<'_>, std::io::Error> {
        match self {
            Body::Empty => Ok(BodyReader::Cursor(Cursor::new(b""))),
            Body::Pending(..) => Err(cannot_read_pending_body_error()),
            Body::Str(s) => Ok(BodyReader::Cursor(Cursor::new(s.as_bytes()))),
            Body::String(s) => Ok(BodyReader::Cursor(Cursor::new(s.as_bytes()))),
            Body::Vec(v) => Ok(BodyReader::Cursor(Cursor::new(v))),
            Body::File(path, ..) => std::fs::File::open(path).map(BodyReader::File),
            Body::TempFile(temp_file, ..) => {
                std::fs::File::open(temp_file.path()).map(BodyReader::File)
            }
        }
    }

    /// # Errors
    /// Returns an error when the body is cached in a file and we fail to open the file.
    #[must_use]
    pub fn async_reader(&self) -> BodyAsyncReader<'_> {
        BodyAsyncReader {
            body: self,
            offset: 0,
            open_fut: None,
            file: None,
        }
    }

    /// # Errors
    /// Returns an error when:
    /// - the request body is not UTF-8
    /// - we fail to read the request body from its temporary file
    pub async fn async_read_as_string(&self) -> Result<String, Response> {
        let mut string = String::new();
        self.async_reader()
            .read_to_string(&mut string)
            .await
            .map_err(|e| match e.kind() {
                ErrorKind::InvalidData => Response::text(400, "Request body is not UTF-8"),
                _ => Response::text(500, "Internal server error: failed reading body file"),
            })?;
        Ok(string)
    }
}
impl From<&'static str> for Body {
    fn from(s: &'static str) -> Self {
        Body::Str(s)
    }
}
impl From<String> for Body {
    fn from(s: String) -> Self {
        Body::String(s)
    }
}
impl From<Vec<u8>> for Body {
    fn from(v: Vec<u8>) -> Self {
        Body::Vec(v)
    }
}
impl<const LEN: usize> From<[u8; LEN]> for Body {
    fn from(b: [u8; LEN]) -> Self {
        Body::Vec(b.to_vec())
    }
}
impl From<&[u8]> for Body {
    fn from(b: &[u8]) -> Self {
        Body::Vec(b.to_vec())
    }
}
impl TryFrom<Body> for String {
    type Error = std::io::Error;

    fn try_from(body: Body) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = body.try_into()?;
        String::from_utf8(bytes)
            .map_err(|_e| std::io::Error::new(ErrorKind::InvalidData, "message body is not UTF-8"))
    }
}
impl TryFrom<Body> for Vec<u8> {
    type Error = std::io::Error;

    fn try_from(body: Body) -> Result<Self, Self::Error> {
        match body {
            Body::Empty => Ok(Vec::new()),
            Body::Pending(..) => Err(cannot_read_pending_body_error()),
            Body::Str(s) => Ok(s.as_bytes().to_vec()),
            Body::String(s) => Ok(s.into_bytes()),
            Body::Vec(v) => Ok(v),
            Body::File(path, ..) => std::fs::read(path),
            Body::TempFile(temp_file, ..) => std::fs::read(temp_file.path()),
        }
    }
}
impl Debug for Body {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            Body::Empty => write!(f, "Body::Empty"),
            Body::Pending(max_len) => write!(f, "Body::Pending(len={:?})", max_len),
            Body::Str(s) => write!(f, "Body::Str({:?})", s),
            Body::String(s) => write!(f, "Body::Str({:?})", s),
            Body::Vec(v) => write!(
                f,
                "Body::Vec({} {:?})",
                v.len(),
                escape_and_elide(v.as_slice(), 100)
            ),
            Body::File(path, len) => {
                write!(f, "Body::File({:?},{})", path.to_string_lossy(), len)
            }
            Body::TempFile(temp_file, len) => write!(
                f,
                "Body::TempFile({:?},{})",
                temp_file.path().to_string_lossy(),
                len
            ),
        }
    }
}

/// # Errors
/// Returns an error when we fail to read the entire request body from the connection
pub async fn read_http_body_to_vec(
    reader: impl AsyncRead + Unpin,
    len: usize,
) -> Result<Body, HttpError> {
    //dbg!("read_http_body_to_vec", len);
    let mut body_vec = Vec::with_capacity(len);
    AsyncReadExt::take(reader, len as u64)
        .read_to_end(&mut body_vec)
        .await
        .map_err(|_e| HttpError::Truncated)?;
    if body_vec.len() < len {
        return Err(HttpError::Truncated);
    }
    Ok(Body::Vec(body_vec))
}

/// # Errors
/// Returns an error when:
/// - the request body is longer than `max_len`
/// - we fail to read the request body from the connection
pub async fn read_http_unsized_body_to_vec(
    mut reader: impl AsyncRead + Unpin,
) -> Result<Body, HttpError> {
    //dbg!("read_http_unsized_body_to_vec");
    let mut body_vec = Vec::new();
    reader
        .read_to_end(&mut body_vec)
        .await
        .map_err(|_| HttpError::Truncated)?;
    Ok(Body::Vec(body_vec))
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
) -> Result<Body, HttpError> {
    //dbg!("read_http_body_to_file", len, dir);
    // TODO: Add async support to `temp_file` and use it here.
    let temp_file =
        TempFile::in_dir(dir).map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    let mut file = async_fs::File::create(temp_file.path())
        .await
        .map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    match copy_async(&mut AsyncReadExt::take(reader, len), &mut file).await {
        CopyResult::Ok(num_copied) if num_copied == len => {}
        CopyResult::Ok(..) | CopyResult::ReaderErr(..) => return Err(HttpError::Truncated),
        CopyResult::WriterErr(e) => {
            return Err(HttpError::ErrorSavingFile(e.kind(), e.to_string()))
        }
    }
    file.close()
        .await
        .map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    Ok(Body::TempFile(temp_file, len))
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
) -> Result<Body, HttpError> {
    //dbg!("read_http_body_to_file", max_len, dir);
    let temp_file =
        TempFile::in_dir(dir).map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    let mut file = async_fs::File::create(temp_file.path())
        .await
        .map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    let len = match copy_async(AsyncReadExt::take(reader, max_len + 1), &mut file).await {
        CopyResult::Ok(len) => len,
        CopyResult::ReaderErr(..) => return Err(HttpError::Truncated),
        CopyResult::WriterErr(e) => {
            return Err(HttpError::ErrorSavingFile(e.kind(), e.to_string()))
        }
    };
    file.close()
        .await
        .map_err(|e| HttpError::ErrorSavingFile(e.kind(), e.to_string()))?;
    if max_len < len {
        return Err(HttpError::BodyTooLong);
    }
    Ok(Body::TempFile(temp_file, len))
}
