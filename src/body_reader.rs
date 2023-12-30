use crate::event::EventReceiver;
use std::io::{Cursor, ErrorKind, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

/// Struct returned by `RequestBody::reader` and `ResponseBody::reader`.
pub enum BodyReader<'x> {
    Cursor(Cursor<&'x [u8]>),
    EventReceiver(&'x Mutex<EventReceiver>),
    File(std::fs::File),
}
impl<'x> BodyReader<'x> {
    #[must_use]
    pub fn bytes(bytes: &'x [u8]) -> Self {
        Self::Cursor(Cursor::new(bytes))
    }

    /// # Errors
    /// Returns an error when it fails to open the file for reading.
    pub fn file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        Ok(Self::File(std::fs::File::open(path.as_ref())?))
    }
}
impl<'x> std::io::Read for BodyReader<'x> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            BodyReader::Cursor(cursor) => cursor.read(buf),
            BodyReader::EventReceiver(mutex_event_receiver) =>
            {
                #[allow(clippy::mut_mutex_lock)]
                mutex_event_receiver.lock().unwrap().read(buf)
            }
            BodyReader::File(file) => file.read(buf),
        }
    }
}
impl<'x> std::io::Seek for BodyReader<'x> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        match self {
            BodyReader::Cursor(cursor) => cursor.seek(pos),
            BodyReader::EventReceiver(..) => Err(std::io::Error::new(
                ErrorKind::Unsupported,
                "BodyReader::EventReceiver cannot seek",
            )),
            BodyReader::File(file) => file.seek(pos),
        }
    }
}
