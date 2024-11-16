use crate::head::read_http_head;
use crate::http_error::HttpError;
use crate::rand::next_insecure_rand_u64;
use crate::{AsciiString, ContentType, HeaderList, RequestBody, Response};
use fixed_buffer::FixedBuf;
use futures_io::AsyncRead;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use url::Url;

#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub remote_addr: SocketAddr,
    pub method: String,
    pub url: Url,
    pub headers: HeaderList,
    pub cookies: HashMap<String, String>,
    pub content_type: ContentType,
    pub expect_continue: bool,
    pub chunked: bool,
    pub gzip: bool,
    pub content_length: Option<u64>,
    pub body: RequestBody,
}
impl Request {
    #[must_use]
    pub fn body(&self) -> &RequestBody {
        &self.body
    }

    #[must_use]
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    #[must_use]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// # Errors
    /// Returns an error when the request body length is known and it is larger than `max_len`.
    ///
    /// When the request body is not known, this returns `Response::get_body_and_reprocess(max_len)`.
    /// The server then tries to read the request body.
    /// If it reads more than `max_len` bytes, it stops and returns `413 Payload Too Large`.
    pub fn recv_body(self, max_len: u64) -> Result<Request, Response> {
        if self.body.len().unwrap_or(0) > max_len {
            Err(Response::payload_too_large_413())
        } else if self.body().is_pending() {
            Err(Response::get_body_and_reprocess(max_len))
        } else {
            Ok(self)
        }
    }

    /// Checks that the request body has type `application/x-www-form-urlencoded`
    /// and deserializes it into type `T`.
    ///
    /// # Errors
    /// Returns an error when:
    /// - the request has no body
    /// - the request body was not received
    /// - the request content type is not `application/x-www-form-urlencoded`
    /// - we fail to parse the body as URL-encoded data
    /// - we fail to deserialize the body into a `T`
    ///
    /// # Panics
    /// Panics when the request body was saved to a file and it fails to read the file.
    #[cfg(feature = "urlencoded")]
    pub fn urlencoded<T: serde::de::DeserializeOwned>(&self) -> Result<T, Response> {
        use crate::util::escape_and_elide;
        use std::io::Read;
        if self.content_type != ContentType::FormUrlEncoded {
            Err(Response::text(
                400,
                "expected x-www-form-urlencoded request body",
            ))
        } else if self.body.is_pending() {
            if self.body.len().is_some() {
                Err(Response::payload_too_large_413())
            } else {
                Err(Response::length_required_411())
            }
        } else {
            let mut buf = Vec::new();
            if let Err(e) = self.body.reader()?.read_to_end(&mut buf) {
                panic!("error reading body: {e}");
            }
            serde_urlencoded::from_bytes(&buf).map_err(|e| {
                Response::text(
                    400,
                    format!(
                        "error processing form data: {}",
                        escape_and_elide(e.to_string().as_bytes(), 100)
                    ),
                )
            })
        }
    }

    /// # Errors
    /// Returns an error when:
    /// - the request has no body
    /// - the request body was not received
    /// - the request content type is not `application/json`
    /// - we fail to parse the body as JSON data
    /// - we fail to deserialize the body into a `T`
    ///
    /// # Panics
    /// Panics when the request body was saved to a file and it fails to read the file.
    #[cfg(feature = "json")]
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, Response> {
        use serde_json::error::Category;
        if self.content_type != ContentType::Json {
            Err(Response::text(400, "expected json request body"))
        } else if self.body.is_pending() {
            if self.body.len().is_some() {
                Err(Response::payload_too_large_413())
            } else {
                Err(Response::length_required_411())
            }
        } else {
            serde_json::from_reader(self.body.reader()?).map_err(|e| match e.classify() {
                Category::Eof => Response::text(400, "truncated json"),
                Category::Io => panic!("error reading body: {e}"),
                Category::Syntax => Response::text(400, format!("malformed json: {e}")),
                Category::Data => Response::text(400, format!("unexpected json: {e}")),
            })
        }
    }

    /// Returns None when the request has no body.
    ///
    /// # Errors
    /// Returns an error when:
    /// - the request body was not received
    /// - the request content type is not `application/json`
    /// - we fail to parse the body as JSON data
    /// - we fail to deserialize the body into a `T`
    ///
    /// # Panics
    /// Panics when the request body was saved to a file and it fails to read the file.
    #[cfg(feature = "json")]
    pub fn opt_json<T: serde::de::DeserializeOwned>(&self) -> Result<Option<T>, Response> {
        if self.body.is_empty().unwrap_or(false) {
            Ok(None)
        } else {
            Ok(Some(self.json()?))
        }
    }

    /// Parses the request URL and deserializes it into type `T`.
    ///
    /// Treats a missing URL query string (`/foo`) as an empty query string (`/foo?`).
    ///
    /// # Errors
    /// Returns an error when
    /// - the URL parameters are mal-formed
    /// - we fail to deserialize the URL parameters into a `T`
    #[cfg(feature = "urlencoded")]
    pub fn parse_url<T: serde::de::DeserializeOwned>(&self) -> Result<T, Response> {
        use crate::util::escape_and_elide;
        let url_str = self.url.query().unwrap_or_default();
        serde_urlencoded::from_str(url_str).map_err(|e| {
            Response::text(
                400,
                format!(
                    "error processing url: {}",
                    escape_and_elide(e.to_string().as_bytes(), 100)
                ),
            )
        })
    }
}
impl Debug for Request {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut cookie_strings: Vec<String> = self
            .cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        cookie_strings.sort();
        write!(
            f,
            "Request{{{}, method={}, path={:?}, headers={:?}, cookies={:?}, {:?}{}{}{}{}, {:?}}}",
            self.remote_addr,
            self.method(),
            self.url().path(),
            self.headers,
            cookie_strings,
            self.content_type(),
            if self.expect_continue { ", expect" } else { "" },
            if self.chunked { ", chunked" } else { "" },
            if self.gzip { ", gzip" } else { "" },
            if let Some(len) = &self.content_length {
                format!(", {len}")
            } else {
                String::new()
            },
            *self.body()
        )
    }
}

/// # Errors
/// Returns an error when:
/// - the connection is closed
/// - we fail to read a full request
/// - we fail to parse the request
/// - the request uses an unsupported transfer encoding
/// - the request content-length is too long to fit in `u64`
#[allow(clippy::module_name_repetitions)]
pub async fn read_http_request<const BUF_SIZE: usize>(
    remote_addr: SocketAddr,
    buf: &mut FixedBuf<BUF_SIZE>,
    reader: impl AsyncRead + Unpin,
) -> Result<Request, HttpError> {
    //dbg!("read_http_request", &buf);
    buf.shift();
    let mut head = read_http_head(buf, reader).await?;
    //dbg!(&head);
    let content_type = head
        .headers
        .remove_only("content-type")
        .map_or(ContentType::None, |s| ContentType::parse(s.as_str()));
    let expect_continue = head
        .headers
        .remove_only("expect")
        .map_or(false, |s| s.as_str() == "100-continue");
    let (gzip, chunked) = {
        let opt_ascii_string = head.headers.remove_only("transfer-encoding");
        let mut iter = opt_ascii_string
            .as_ref()
            .map(AsciiString::as_str)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match (iter.next(), iter.next(), iter.next()) {
            (Some("gzip"), Some("chunked"), None) => (true, true),
            (Some("gzip"), None, None) => (true, false),
            (Some("chunked"), None, None) => (false, true),
            (None, None, None) => (false, false),
            _ => return Err(HttpError::UnsupportedTransferEncoding),
        }
    };
    let mut cookies = HashMap::new();
    for header_value in head.headers.get_all("cookie") {
        for cookie_str in header_value
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let mut parts = cookie_str.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(name), Some(value)) => {
                    cookies.insert(name.to_string(), value.to_string());
                }
                _ => return Err(HttpError::MalformedCookieHeader),
            }
        }
    }
    let content_length = if let Some(s) = head.headers.get_only("content-length") {
        Some(s.parse().map_err(|_| HttpError::InvalidContentLength)?)
    } else {
        None
    };
    #[allow(clippy::match_same_arms)]
    // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3
    let body = match (chunked, &content_length, head.method.as_str()) {
        (true, _, _) => RequestBody::PendingUnknown,
        (false, Some(0), _) => RequestBody::empty(),
        (false, Some(len), _) => RequestBody::PendingKnown(*len),
        (false, None, "POST" | "PUT") => RequestBody::PendingUnknown,
        (false, None, _) if expect_continue || gzip => RequestBody::PendingUnknown,
        (false, None, _) => RequestBody::empty(),
    };
    Ok(Request {
        id: next_insecure_rand_u64(),
        remote_addr,
        method: head.method,
        url: head.url,
        headers: head.headers,
        cookies,
        content_type,
        expect_continue,
        chunked,
        gzip,
        content_length,
        body,
    })
}
