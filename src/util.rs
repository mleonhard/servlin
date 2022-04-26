use fixed_buffer::FixedBuf;
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum CopyResult {
    Ok(u64),
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
}
impl CopyResult {
    #[allow(clippy::missing_errors_doc)]
    pub fn map_errs<E>(
        self,
        read_err_op: impl FnOnce(std::io::Error) -> E,
        write_err_op: impl FnOnce(std::io::Error) -> E,
    ) -> Result<u64, E> {
        match self {
            CopyResult::Ok(n) => Ok(n),
            CopyResult::ReaderErr(e) => Err(read_err_op(e)),
            CopyResult::WriterErr(e) => Err(write_err_op(e)),
        }
    }
}

/// Copies bytes from `reader` to `writer`.
pub async fn copy_async(
    mut reader: impl AsyncRead + Unpin,
    mut writer: impl AsyncWrite + Unpin,
) -> CopyResult {
    let mut buf = <FixedBuf<65536>>::new();
    let mut num_copied = 0;
    loop {
        match reader.read(buf.writable()).await {
            Ok(0) => return CopyResult::Ok(num_copied),
            Ok(n) => buf.wrote(n),
            Err(e) => return CopyResult::ReaderErr(e),
        }
        let readable = buf.read_all();
        match writer.write_all(readable).await {
            Ok(()) => num_copied += readable.len() as u64,
            Err(e) => return CopyResult::WriterErr(e),
        }
    }
}

fn hex_digit(n: u8) -> u8 {
    match n {
        0 => b'0',
        1 => b'1',
        2 => b'2',
        3 => b'3',
        4 => b'4',
        5 => b'5',
        6 => b'6',
        7 => b'7',
        8 => b'8',
        9 => b'9',
        10 => b'a',
        11 => b'b',
        12 => b'c',
        13 => b'd',
        14 => b'e',
        15 => b'f',
        _ => unimplemented!(),
    }
}

fn trim_prefix(mut slice: &[u8], prefix: u8) -> &[u8] {
    while !slice.is_empty() && slice[0] == prefix {
        slice = &slice[1..];
    }
    slice
}

/// Reads blocks from `reader`, encodes them in HTTP chunked encoding, and writes them to `writer`.
#[allow(clippy::missing_panics_doc)]
pub async fn copy_chunked_async(
    mut reader: impl AsyncRead + Unpin,
    mut writer: impl AsyncWrite + Unpin,
) -> CopyResult {
    let mut num_copied = 0;
    loop {
        let mut buf = [0_u8; 65536];
        let len = match reader.read(&mut buf[6..65534]).await {
            Ok(0) => break,
            Ok(len) => len,
            Err(e) => return CopyResult::ReaderErr(e),
        };
        buf[0] = hex_digit(u8::try_from((len >> 12) & 0xF).unwrap());
        buf[1] = hex_digit(u8::try_from((len >> 8) & 0xF).unwrap());
        buf[2] = hex_digit(u8::try_from((len >> 4) & 0xF).unwrap());
        buf[3] = hex_digit(u8::try_from(len & 0xF).unwrap());
        buf[4] = b'\r';
        buf[5] = b'\n';
        buf[6 + len] = b'\r';
        buf[6 + len + 1] = b'\n';
        let bytes = &buf[..(6 + len + 2)];
        let bytes = trim_prefix(bytes, b'0');
        if let Err(e) = writer.write_all(bytes).await {
            return CopyResult::WriterErr(e);
        }
        num_copied += len as u64;
    }
    if let Err(e) = writer.write_all(b"0\r\n\r\n").await {
        return CopyResult::WriterErr(e);
    }
    num_copied += 3;
    CopyResult::Ok(num_copied)
}

/// Convert a byte slice into a string.
/// Includes printable ASCII characters as-is.
/// Converts non-printable or non-ASCII characters to strings like "\n" and "\x19".
///
/// Uses
/// [`core::ascii::escape_default`](https://doc.rust-lang.org/core/ascii/fn.escape_default.html)
/// internally to escape each byte.
///
/// This function is useful for printing byte slices to logs and comparing byte slices in tests.
///
/// Example test:
/// ```
/// use fixed_buffer::escape_ascii;
/// assert_eq!("abc", escape_ascii(b"abc"));
/// assert_eq!("abc\\n", escape_ascii(b"abc\n"));
/// assert_eq!(
///     "Euro sign: \\xe2\\x82\\xac",
///     escape_ascii("Euro sign: \u{20AC}".as_bytes())
/// );
/// assert_eq!("\\x01\\x02\\x03", escape_ascii(&[1, 2, 3]));
/// ```
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn escape_ascii(input: &[u8]) -> String {
    let mut result = String::new();
    for byte in input {
        for ascii_byte in core::ascii::escape_default(*byte) {
            result.push_str(core::str::from_utf8(&[ascii_byte]).unwrap());
        }
    }
    result
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn escape_and_elide(input: &[u8], max_len: usize) -> String {
    if input.len() > max_len {
        escape_ascii(&input[..max_len]) + "..."
    } else {
        escape_ascii(input)
    }
}

pub fn find_slice<T: std::cmp::PartialEq>(needle: &[T], haystack: &[T]) -> Option<usize> {
    if needle.len() <= haystack.len() {
        for n in 0..=(haystack.len() - needle.len()) {
            if &haystack[n..(n + needle.len())] == needle {
                return Some(n);
            }
        }
    }
    None
}

/// Wraps an `AsyncWrite` and records the number of bytes successfully written to it.
pub struct AsyncWriteCounter<W>(W, u64);
impl<W: AsyncWrite + Unpin> AsyncWriteCounter<W> {
    pub fn new(writer: W) -> Self {
        Self(writer, 0)
    }

    pub fn num_bytes_written(&self) -> u64 {
        self.1
    }
}
impl<W: AsyncWrite + Unpin> AsyncWrite for AsyncWriteCounter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match Pin::new(&mut self.0).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                self.1 += n as u64;
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_close(cx)
    }
}
