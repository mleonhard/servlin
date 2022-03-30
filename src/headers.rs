use crate::util::escape_and_elide;
use crate::AsciiString;
use core::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Header {
    pub name: AsciiString,
    pub value: AsciiString,
}
impl Header {
    #[must_use]
    pub fn new(name: AsciiString, value: AsciiString) -> Self {
        Self { name, value }
    }
}
impl Debug for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "Header({}:{})",
            escape_and_elide(self.name.as_bytes(), 30),
            escape_and_elide(self.value.as_bytes(), 1000)
        )
    }
}
impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "{}:{}", self.name.as_str(), self.value.as_str())
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct HeaderList(pub Vec<Header>);
impl HeaderList {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a header.
    ///
    /// You can call this multiple times to add multiple headers with the same name.
    ///
    /// The [HTTP spec](https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.4)
    /// limits header names to US-ASCII and header values to US-ASCII or ISO-8859-1.
    ///
    /// # Panics
    /// Panics when `name` is not US-ASCII.
    pub fn add(&mut self, name: impl AsRef<str>, value: AsciiString) {
        self.0
            .push(Header::new(name.as_ref().try_into().unwrap(), value));
    }

    /// Searches for a header that matches `name`.
    /// Uses a case-insensitive comparison.
    ///
    /// Returns the value of the header.
    ///
    /// Returns `None` when multiple headers matched or none matched.
    pub fn get_only(&self, name: impl AsRef<str>) -> Option<&AsciiString> {
        let mut value = None;
        for header in &self.0 {
            if header.name.eq_ignore_ascii_case(name.as_ref()) {
                if value.is_some() {
                    return None;
                }
                value = Some(&header.value);
            }
        }
        value
    }

    /// Looks for headers with names that match `name`.
    /// Uses a case-insensitive comparison.
    /// Returns the values of the matching headers.
    pub fn get_all(&self, name: impl AsRef<str>) -> Vec<&AsciiString> {
        let mut headers = Vec::new();
        for header in &self.0 {
            if header.name.eq_ignore_ascii_case(name.as_ref()) {
                headers.push(&header.value);
            }
        }
        headers
    }

    /// Removes all headers with the specified `name`.
    /// Uses a case-insensitive comparison.
    ///
    /// When only one header matched, returns the value of that header.
    ///
    /// Returns `None` when multiple headers matched or none matched.
    pub fn remove_only(&mut self, name: impl AsRef<str>) -> Option<AsciiString> {
        let mut iter = self.remove_all(name).into_iter();
        match (iter.next(), iter.next()) {
            (Some(value), None) => Some(value),
            _ => None,
        }
    }

    /// Removes all headers with the specified `name`.
    /// Uses a case-insensitive comparison.
    ///
    /// Returns the values of the headers.
    pub fn remove_all(&mut self, name: impl AsRef<str>) -> Vec<AsciiString> {
        let mut values = Vec::new();
        let mut n = 0;
        while n < self.0.len() {
            if self.0[n].name.eq_ignore_ascii_case(name.as_ref()) {
                let header = self.0.swap_remove(n);
                values.push(header.value);
            } else {
                n += 1;
            }
        }
        values
    }
}
impl Debug for HeaderList {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut strings: Vec<String> = self
            .iter()
            .map(|h| format!("{}: {:?}", h.name, h.value.as_str()))
            .collect();
        strings.sort();
        write!(f, "{{{}}}", strings.join(", "),)
    }
}
impl Default for HeaderList {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for HeaderList {
    type Target = Vec<Header>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for HeaderList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'x> IntoIterator for &'x HeaderList {
    type Item = &'x Header;
    type IntoIter = core::slice::Iter<'x, Header>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'x> IntoIterator for &'x mut HeaderList {
    type Item = &'x mut Header;
    type IntoIter = core::slice::IterMut<'x, Header>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
