use crate::time::FormatTime;
use crate::AsciiString;
use core::fmt::{Display, Formatter};
use core::time::Duration;
use std::time::SystemTime;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cookie {
    name: AsciiString,
    value: AsciiString,
    domain: AsciiString,
    expires: SystemTime,
    http_only: bool,
    path: AsciiString,
    max_age: Duration,
    same_site: SameSite,
    secure: bool,
}
impl Cookie {
    /// Makes a new cookie with the specified name and value.
    ///
    /// The cookie has `max_age` set to 30 days,
    /// `same_site` strict,
    /// `secure` true,
    /// and `http_only` true.
    ///
    /// # Panics
    /// Panics when `name` is empty.
    /// Panics when `name` is not US-ASCII.
    #[must_use]
    pub fn new(name: impl AsRef<str>, value: AsciiString) -> Self {
        assert!(
            !name.as_ref().is_empty(),
            "Cookie::new called with empty `name`"
        );
        Self {
            name: name.as_ref().to_string().try_into().unwrap(),
            value,
            domain: AsciiString::new(),
            expires: SystemTime::UNIX_EPOCH,
            http_only: true,
            max_age: Duration::from_secs(30 * 24 * 60 * 60),
            path: AsciiString::new(),
            same_site: SameSite::Strict,
            secure: true,
        }
    }

    /// # Panics
    /// Panics when `domain` is not US-ASCII.
    #[must_use]
    pub fn with_domain(mut self, d: impl AsRef<str>) -> Self {
        self.domain = d.as_ref().try_into().unwrap();
        self
    }

    /// To un-set this value, pass `SystemTime::UNIX_EPOCH`.
    #[must_use]
    pub fn with_expires(mut self, t: SystemTime) -> Self {
        self.expires = t;
        self
    }

    #[must_use]
    pub fn with_http_only(mut self, b: bool) -> Self {
        self.http_only = b;
        self
    }

    /// To un-set duration, pass `Duration::ZERO`.
    #[must_use]
    pub fn with_max_age(mut self, d: Duration) -> Self {
        self.max_age = d;
        self
    }

    /// # Panics
    /// Panics when `p` is not US-ASCII.
    #[must_use]
    pub fn with_path(mut self, p: impl AsRef<str>) -> Self {
        self.path = p.as_ref().try_into().unwrap();
        self
    }

    #[must_use]
    pub fn with_same_site(mut self, s: SameSite) -> Self {
        self.same_site = s;
        self
    }

    #[must_use]
    pub fn with_secure(mut self, b: bool) -> Self {
        self.secure = b;
        self
    }
}
impl Display for Cookie {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
        // Set-Cookie: <cookie-name>=<cookie-value>
        // Set-Cookie: <cookie-name>=<cookie-value>; Expires=<date>
        // Set-Cookie: <cookie-name>=<cookie-value>; Max-Age=<number>
        // Set-Cookie: <cookie-name>=<cookie-value>; Domain=<domain-value>
        // Set-Cookie: <cookie-name>=<cookie-value>; Path=<path-value>
        // Set-Cookie: <cookie-name>=<cookie-value>; Secure
        // Set-Cookie: <cookie-name>=<cookie-value>; HttpOnly
        //
        // Set-Cookie: <cookie-name>=<cookie-value>; SameSite=Strict
        // Set-Cookie: <cookie-name>=<cookie-value>; SameSite=Lax
        // Set-Cookie: <cookie-name>=<cookie-value>; SameSite=None; Secure
        //
        // // Multiple attributes are also possible, for example:
        // Set-Cookie: <cookie-name>=<cookie-value>; Domain=<domain-value>; Secure; HttpOnly
        write!(f, "{}={}", self.name.as_str(), self.value.as_str())?;
        if !self.domain.is_empty() {
            write!(f, "; Domain={}", self.domain.as_str())?;
        }
        if self.expires != SystemTime::UNIX_EPOCH {
            write!(f, "; Expires={}", self.expires.iso8601_utc())?;
        }
        if self.http_only {
            write!(f, "; HttpOnly")?;
        }
        if self.max_age > Duration::ZERO {
            write!(f, "; Max-Age={}", self.max_age.as_secs())?;
        }
        if !self.path.is_empty() {
            write!(f, "; Path={}", self.path.as_str())?;
        }
        match self.same_site {
            SameSite::Strict => write!(f, "; SameSite=Strict")?,
            SameSite::Lax => write!(f, "; SameSite=Lax")?,
            SameSite::None => write!(f, "; SameSite=None")?,
        }
        if self.secure {
            write!(f, "; Secure")?;
        }
        Ok(())
    }
}
impl From<Cookie> for AsciiString {
    fn from(cookie: Cookie) -> Self {
        AsciiString::try_from(format!("{cookie}")).unwrap()
    }
}
