use crate::util::escape_and_elide;
use core::borrow::Borrow;
use core::fmt::{Display, Formatter};
use std::borrow::Cow;
use std::ops::Deref;

/// Wraps a [`String`] that contains only US-ASCII chars.
///
/// Implements [`Deref`] so you can access the internal string directly.
///
/// Implements [`From`] for various numeric types.
///
/// Implements [`TryFrom`] for various string types.
///
/// Example:
/// ```
/// use beatrice::AsciiString;
/// use core::convert::TryInto;
///
/// let value1: AsciiString = "value1".try_into().unwrap();
/// let value2: AsciiString = 123_usize.into();
/// // Call `String::as_str` via the `Deref` implementation.
/// let value2_str = value2.as_str();
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AsciiString(String);
impl AsciiString {
    #[must_use]
    pub fn new() -> Self {
        Self(String::new())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl AsRef<[u8]> for AsciiString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}
impl AsRef<str> for AsciiString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
impl Borrow<str> for AsciiString {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}
impl Default for AsciiString {
    fn default() -> Self {
        Self::new()
    }
}
impl<'x> Deref for AsciiString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Display for AsciiString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "{}", self.0)
    }
}
impl From<AsciiString> for String {
    fn from(ascii_string: AsciiString) -> Self {
        ascii_string.0
    }
}

impl From<i8> for AsciiString {
    fn from(n: i8) -> Self {
        Self(n.to_string())
    }
}
impl From<u8> for AsciiString {
    fn from(n: u8) -> Self {
        Self(n.to_string())
    }
}
impl From<i16> for AsciiString {
    fn from(n: i16) -> Self {
        Self(n.to_string())
    }
}
impl From<u16> for AsciiString {
    fn from(n: u16) -> Self {
        Self(n.to_string())
    }
}
impl From<i32> for AsciiString {
    fn from(n: i32) -> Self {
        Self(n.to_string())
    }
}
impl From<u32> for AsciiString {
    fn from(n: u32) -> Self {
        Self(n.to_string())
    }
}
impl From<i64> for AsciiString {
    fn from(n: i64) -> Self {
        Self(n.to_string())
    }
}
impl From<u64> for AsciiString {
    fn from(n: u64) -> Self {
        Self(n.to_string())
    }
}
impl From<usize> for AsciiString {
    fn from(n: usize) -> Self {
        Self(n.to_string())
    }
}

fn try_from_error(bytes: impl AsRef<[u8]>) -> String {
    format!(
        "`AsciiString::try_from` called with non-ASCII value: \"{}\"",
        escape_and_elide(bytes.as_ref(), 100)
    )
}

impl TryFrom<char> for AsciiString {
    type Error = String;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        if c.is_ascii() {
            Ok(Self(c.to_string()))
        } else {
            Err(try_from_error(c.to_string()))
        }
    }
}

impl TryFrom<String> for AsciiString {
    type Error = String;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        if string.is_ascii() {
            Ok(Self(string))
        } else {
            Err(try_from_error(string))
        }
    }
}

impl TryFrom<&String> for AsciiString {
    type Error = String;

    fn try_from(string: &String) -> Result<Self, Self::Error> {
        if string.is_ascii() {
            Ok(Self(string.to_string()))
        } else {
            Err(try_from_error(string))
        }
    }
}

impl TryFrom<&str> for AsciiString {
    type Error = String;

    fn try_from(str: &str) -> Result<Self, Self::Error> {
        if str.is_ascii() {
            Ok(Self(str.to_string()))
        } else {
            Err(try_from_error(str))
        }
    }
}

impl TryFrom<&mut str> for AsciiString {
    type Error = String;

    fn try_from(mut_str: &mut str) -> Result<Self, Self::Error> {
        if mut_str.is_ascii() {
            Ok(Self((*mut_str).to_string()))
        } else {
            Err(try_from_error(mut_str))
        }
    }
}

impl TryFrom<Box<str>> for AsciiString {
    type Error = String;

    fn try_from(box_str: Box<str>) -> Result<Self, Self::Error> {
        if box_str.is_ascii() {
            Ok(Self(box_str.to_string()))
        } else {
            Err(try_from_error(box_str.as_bytes()))
        }
    }
}

impl<'x> TryFrom<Cow<'x, str>> for AsciiString {
    type Error = String;

    fn try_from(cow_str: Cow<'x, str>) -> Result<Self, Self::Error> {
        if cow_str.is_ascii() {
            Ok(Self(cow_str.to_string()))
        } else {
            Err(try_from_error(cow_str.as_bytes()))
        }
    }
}
