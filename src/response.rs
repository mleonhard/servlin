use futures_io::AsyncWrite;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::io::Write;

use crate::http_error::HttpError;
use crate::util::{copy_async, CopyResult};
use crate::{Body, ContentType, Request};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;

// TODO: Rename to HttpBody.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyWrapper<'x>(&'x Body);
impl<'x> Deref for BodyWrapper<'x> {
    type Target = &'x Body;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Response {
    Drop,
    /// `GetBodyAndReprocess(max_len: u64, Request)`<br>
    /// Read the body from the client, but only up to the specified `u64` bytes.
    GetBodyAndReprocess(u64, Request),
    /// Normal(code: u16, ContentType, headers: HashMap<String, String>, Body)
    Normal(u16, ContentType, HashMap<String, String>, Body),
}
impl Response {
    #[must_use]
    pub fn new(code: u16) -> Self {
        Response::Normal(code, ContentType::None, HashMap::new(), Body::empty())
    }

    /// # Errors
    /// Returns an error when it fails to serialize `v`.
    #[cfg(feature = "json")]
    pub fn json(code: u16, v: impl serde::Serialize) -> Result<Response, String> {
        let body_vec = serde_json::to_vec(&v)
            .map_err(|e| format!("error serializing response to json: {}", e))?;
        Ok(Response::Normal(
            code,
            ContentType::Json,
            HashMap::new(),
            Body::Vec(body_vec),
        ))
    }

    #[must_use]
    pub fn text(code: u16, body: impl Into<Body>) -> Self {
        Response::Normal(code, ContentType::PlainText, HashMap::new(), body.into())
    }

    #[must_use]
    pub fn method_not_allowed_405(allowed_methods: &[&'static str]) -> Self {
        Response::Normal(
            405,
            ContentType::None,
            [("allow".to_string(), allowed_methods.join(","))].into(),
            Body::empty(),
        )
    }

    #[must_use]
    pub fn payload_too_large_413() -> Self {
        Response::text(413, "Uploaded data is too big.")
    }

    #[must_use]
    fn into_tuple(self) -> (u16, ContentType, HashMap<String, String>, Body) {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(c, t, h, b) => (c, t, h, b),
        }
    }

    #[must_use]
    pub fn with_body(self, b: impl Into<Body>) -> Self {
        let (c, t, h, _b) = self.into_tuple();
        Response::Normal(c, t, h, b.into())
    }

    /// To use this method, enable cargo feature `"internals"`.
    #[cfg(feature = "internals")]
    #[must_use]
    pub fn with_http_body(self, b: impl Into<Body>) -> Self {
        let (c, t, h, _b) = self.into_tuple();
        Response::Normal(c, t, h, b.into())
    }

    #[must_use]
    pub fn with_header(self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let (c, t, mut h, b) = self.into_tuple();
        h.insert(
            name.as_ref().to_ascii_lowercase(),
            value.as_ref().to_string(),
        );
        Response::Normal(c, t, h, b)
    }

    #[must_use]
    pub fn with_status(self, c: u16) -> Self {
        let (_c, t, h, b) = self.into_tuple();
        Response::Normal(c, t, h, b)
    }

    #[must_use]
    pub fn with_type(self, t: ContentType) -> Self {
        let (c, _t, h, b) = self.into_tuple();
        Response::Normal(c, t, h, b)
    }

    #[must_use]
    pub fn is_normal(&self) -> bool {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => false,
            Response::Normal(..) => true,
        }
    }

    // TODO: Change this to return Option<u16>.
    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn code(&self) -> u16 {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(c, _t, _h, _b) => *c,
        }
    }

    #[must_use]
    pub fn is_1xx(&self) -> bool {
        self.code() / 100 == 1
    }

    #[must_use]
    pub fn is_2xx(&self) -> bool {
        self.code() / 100 == 2
    }

    #[must_use]
    pub fn is_3xx(&self) -> bool {
        self.code() / 100 == 3
    }

    #[must_use]
    pub fn is_4xx(&self) -> bool {
        self.code() / 100 == 4
    }

    #[must_use]
    pub fn is_5xx(&self) -> bool {
        self.code() / 100 == 5
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn reason_phrase(&self) -> &'static str {
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
        match self.code() {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choice",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required ",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a teapot",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early ",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            _ => "Response",
        }
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn content_type(&self) -> &ContentType {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, t, _h, _b) => t,
        }
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn headers(&self) -> &HashMap<String, String> {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, _t, h, _b) => h,
        }
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn body(&self) -> BodyWrapper<'_> {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, _t, _h, b) => BodyWrapper(b),
        }
    }

    /// To use this method, enable cargo feature `"internals"`.
    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[cfg(feature = "internals")]
    #[must_use]
    pub fn internal_body(&self) -> &Body {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, _t, _h, b) => b,
        }
    }
}
impl From<std::io::Error> for Response {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            ErrorKind::InvalidData => Response::text(400, "Bad request"),
            _ => Response::text(500, "Internal server error"),
        }
    }
}
impl Debug for Response {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            Response::Drop => write!(f, "Response::Drop"),
            Response::GetBodyAndReprocess(max_len, _req) => {
                write!(f, "Response::GetBodyAndReprocess({},..)", max_len)
            }
            Response::Normal(.., body) => {
                let mut headers: Vec<String> = self
                    .headers()
                    .iter()
                    .map(|(n, v)| format!("{}: {:?}", n, v))
                    .collect();
                headers.sort();
                write!(
                    f,
                    "Response::Normal({} {}, {:?}, headers={{{}}}, {:?})",
                    self.code(),
                    self.reason_phrase(),
                    self.content_type(),
                    headers.join(", "),
                    body
                )
            }
        }
    }
}

/// # Errors
/// Returns an error when:
/// - `response` is not `Response::Normal`
/// - the connection is closed
/// - we fail to send the response on the connection
/// - the response body is saved in a file and we fail to read the file
#[allow(clippy::module_name_repetitions)]
pub async fn write_http_response(
    mut writer: impl AsyncWrite + Unpin,
    response: &Response,
) -> Result<(), HttpError> {
    //dbg!("write_http_response", &response);
    if !response.is_normal() {
        return Err(HttpError::UnwritableResponse);
    }
    // https://datatracker.ietf.org/doc/html/rfc7230#section-3.1.2
    //     status-line = HTTP-version SP status-code SP reason-phrase CRLF
    //     status-code    = 3DIGIT
    //     reason-phrase  = *( HTAB / SP / VCHAR )
    let mut head_bytes: Vec<u8> = format!(
        "HTTP/1.1 {} {}\r\n",
        response.code(),
        response.reason_phrase()
    )
    .into_bytes();
    if response.content_type() != &ContentType::None {
        if response.headers().contains_key("content-type") {
            return Err(HttpError::DuplicateContentTypeHeader);
        }
        write!(
            head_bytes,
            "content-type: {}\r\n",
            response.content_type().as_str()
        )
        .unwrap();
    }
    if response.body().len() > 0 {
        if response.headers().contains_key("content-length") {
            return Err(HttpError::DuplicateContentLengthHeader);
        }
        write!(head_bytes, "content-length: {}\r\n", response.body().len()).unwrap();
    }
    for (name, value) in response.headers() {
        // Convert headers from UTF-8 back to ISO-8859-1, with 0xFF for a replacement byte.
        write!(head_bytes, "{}: ", name).unwrap();
        head_bytes.extend(value.chars().map(|c| u8::try_from(c).unwrap_or(255)));
        head_bytes.extend(b"\r\n");
    }
    head_bytes.extend(b"\r\n");
    writer
        .write_all(head_bytes.as_slice())
        .await
        .map_err(|_| HttpError::Disconnected)?;
    drop(head_bytes);
    if response.body().len() > 0 {
        //dbg!(response.body().len());
        match copy_async(
            AsyncReadExt::take(response.body().async_reader(), response.body().len()),
            &mut writer,
        )
        .await
        {
            CopyResult::Ok(len) if len == response.body().len() => {
                //dbg!(len);
            }
            CopyResult::Ok(_len) => {
                return Err(HttpError::ErrorReadingFile(
                    ErrorKind::UnexpectedEof,
                    "body file is smaller than expected".to_string(),
                ))
            }
            CopyResult::ReaderErr(e) => {
                return Err(HttpError::ErrorReadingFile(e.kind(), e.to_string()))
            }
            CopyResult::WriterErr(..) => return Err(HttpError::Disconnected),
        };
    }
    let result = writer.flush().await.map_err(|_| HttpError::Disconnected);
    //dbg!(&result);
    result
}
