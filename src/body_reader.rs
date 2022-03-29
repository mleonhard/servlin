use std::io::Cursor;
use std::path::Path;

/// Struct returned by `RequestBody::reader` and `ResponseBody::reader`.
pub enum BodyReader<'x> {
    Cursor(Cursor<&'x [u8]>),
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
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BodyReader::Cursor(cursor) => cursor.read(buf),
            BodyReader::File(file) => file.read(buf),
        }
    }
}
