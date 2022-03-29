use crate::head::read_http_head;
use crate::http_error::HttpError;
use crate::{Body, ContentType, Response};
use fixed_buffer::FixedBuf;
use futures_io::AsyncRead;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::net::SocketAddr;
use url::Url;

#[cfg(feature = "internals")]
#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub remote_addr: SocketAddr,
    pub method: String,
    pub url: Url,
    pub headers_lowercase: HashMap<String, String>,
    pub content_type: ContentType,
    pub expect_continue: bool,
    pub chunked: bool,
    pub gzip: bool,
    pub content_length: Option<u64>,
    pub body: Body,
}
#[cfg(not(feature = "internals"))]
#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub(crate) remote_addr: SocketAddr,
    pub(crate) method: String,
    pub(crate) url: Url,
    pub(crate) headers_lowercase: HashMap<String, String>,
    pub(crate) content_type: ContentType,
    pub(crate) expect_continue: bool,
    pub(crate) chunked: bool,
    pub(crate) gzip: bool,
    pub(crate) content_length: Option<u64>,
    pub(crate) body: Body,
}
impl Request {
    #[must_use]
    pub fn body(&self) -> &Body {
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

    #[must_use]
    pub fn header(&self, name_lowercase: impl AsRef<str>) -> Option<&str> {
        self.headers_lowercase
            .get(name_lowercase.as_ref())
            .map(String::as_str)
    }

    /// # Errors
    /// Returns an error when the request body length is known and it is larger than `max_len`.
    ///
    /// When the request body is not known, this returns `Response::GetBodyAndReprocess`.
    /// The connection handler (the internal `HttpConn` struct) then tries to read the request body.
    /// If it reads more than `max_len` bytes, it stops and returns `413 Payload Too Large`.
    pub fn recv_body(self, max_len: u64) -> Result<Request, Response> {
        if self.body.len() > max_len {
            Err(Response::payload_too_large_413())
        } else if self.body().is_pending() {
            Err(Response::GetBodyAndReprocess(max_len, self))
        } else {
            Ok(self)
        }
    }

    /// # Errors
    /// Returns an error when it fails to parse the request body as JSON and deserialize it
    /// into a `T`.
    ///
    /// # Panics
    /// Panics when the request body was saved to a file and it fails to read the file.
    #[cfg(feature = "json")]
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, Response> {
        use serde_json::error::Category;
        if self.content_type != ContentType::Json {
            Err(Response::text(400, "expected json request body"))
        } else if self.body.is_pending() {
            if self.body.length_is_known() {
                Err(Response::payload_too_large_413())
            } else {
                Err(Response::length_required_411())
            }
        } else {
            serde_json::from_reader(self.body.reader()?).map_err(|e| match e.classify() {
                Category::Eof => Response::text(400, "truncated json"),
                Category::Io => panic!("error reading body: {}", e),
                Category::Syntax => Response::text(400, format!("malformed json: {}", e)),
                Category::Data => Response::text(400, format!("unexpected json: {}", e)),
            })
        }
    }
}
impl Debug for Request {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut headers: Vec<String> = self
            .headers_lowercase
            .iter()
            .map(|(n, v)| format!("{}: {:?}", n, v))
            .collect();
        headers.sort();
        write!(
            f,
            "Request{{{}, {}, {:?}, headers={{{}}}, {:?}{}{}{}, {:?}}}",
            self.remote_addr,
            self.method(),
            self.url().path(),
            headers.join(", "),
            self.content_type(),
            if self.expect_continue { ", expect" } else { "" },
            if self.chunked { ", chunked" } else { "" },
            if let Some(len) = &self.content_length {
                format!(", {}", len)
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
    //dbg!("read_http_request");
    buf.shift();
    let head = read_http_head(buf, reader).await?;
    let content_type = head
        .headers_lowercase
        .get("content-type")
        .map_or(ContentType::None, |s| ContentType::parse(s));
    let expect_continue = head
        .headers_lowercase
        .get("expect")
        .map_or(false, |s| s == "100-continue");
    let transfer_encoding = head
        .headers_lowercase
        .get("transfer-encoding")
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut transfer_encodings = transfer_encoding
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<HashSet<&str>>();
    let chunked = transfer_encodings.remove("chunked");
    let gzip = transfer_encodings.remove("gzip");
    if !transfer_encodings.is_empty() {
        return Err(HttpError::UnsupportedTransferEncoding);
    }
    let content_length = if let Some(s) = head.headers_lowercase.get("content-length") {
        Some(s.parse().map_err(|_| HttpError::InvalidContentLength)?)
    } else {
        None
    };
    #[allow(clippy::match_same_arms)]
    // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3
    let body = match (chunked, &content_length, head.method.as_str()) {
        (true, _, _) => Body::PendingUnknown,
        (false, Some(0), _) => Body::empty(),
        (false, Some(len), _) => Body::PendingKnown(*len),
        (false, None, "POST" | "PUT") => Body::PendingUnknown,
        (false, None, _) if expect_continue || gzip => Body::PendingUnknown,
        (false, None, _) => Body::empty(),
    };
    Ok(Request {
        remote_addr,
        method: head.method,
        url: head.url,
        headers_lowercase: head.headers_lowercase,
        content_type,
        expect_continue,
        chunked,
        gzip,
        content_length,
        body,
    })
}
