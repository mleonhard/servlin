use std::io::{Cursor, Read};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Struct returned by `RequestBody::async_reader` and `ResponseBody::async_reader`.
pub enum BodyAsyncReader<'x> {
    Cursor(Cursor<&'x [u8]>),
    File(async_fs::File),
}
impl<'x> BodyAsyncReader<'x> {
    #[must_use]
    pub fn bytes(bytes: &'x [u8]) -> Self {
        Self::Cursor(Cursor::new(bytes))
    }

    /// # Errors
    /// Returns an error when it fails to open the file for reading.
    pub async fn file(path: impl AsRef<Path>) -> Result<BodyAsyncReader<'x>, std::io::Error> {
        let file = async_fs::File::open(path.as_ref()).await?;
        Ok(BodyAsyncReader::File(file))
    }
}
impl<'x> futures_io::AsyncRead for BodyAsyncReader<'x> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            BodyAsyncReader::Cursor(cursor) => Poll::Ready(cursor.read(buf)),
            BodyAsyncReader::File(async_fs_file) => Pin::new(async_fs_file).poll_read(cx, buf),
        }
    }
}
