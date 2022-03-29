use futures_io::AsyncWrite;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::io::Write;

use crate::http_error::HttpError;
use crate::util::{copy_async, CopyResult};
use crate::{AsciiString, ContentType, ResponseBody};
use std::fmt::Debug;

#[allow(clippy::module_name_repetitions)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResponseKind {
    DropConnection,
    /// `GetBodyAndReprocess(max_len: u64)`<br>
    /// Read the body from the client, but only up to the specified `u64` bytes.
    GetBodyAndReprocess(u64),
    Normal,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Response {
    pub kind: ResponseKind,
    pub code: u16,
    pub content_type: ContentType,
    /// The [HTTP spec](https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.4)
    /// limits header names to US-ASCII and header values to US-ASCII or ISO-8859-1.
    pub headers: Vec<(AsciiString, AsciiString)>,
    pub body: ResponseBody,
}
impl Response {
    #[must_use]
    pub fn new(code: u16) -> Self {
        Self {
            kind: ResponseKind::Normal,
            code,
            content_type: ContentType::None,
            headers: Vec::new(),
            body: ResponseBody::empty(),
        }
    }

    /// Return this and the server will drop the connection.
    #[must_use]
    pub fn drop_connection() -> Self {
        Self {
            kind: ResponseKind::DropConnection,
            code: 0,
            content_type: ContentType::None,
            headers: Vec::new(),
            body: ResponseBody::empty(),
        }
    }

    /// Return this and the server will read the request body from the client
    /// and call the request handler again.
    ///
    /// If the request body is larger than `max_len` bytes, it sends 413 Payload Too Large.
    #[must_use]
    pub fn get_body_and_reprocess(max_len: u64) -> Self {
        Self {
            kind: ResponseKind::GetBodyAndReprocess(max_len),
            code: 0,
            content_type: ContentType::None,
            headers: Vec::new(),
            body: ResponseBody::empty(),
        }
    }

    #[must_use]
    pub fn html(code: u16, body: impl Into<ResponseBody>) -> Self {
        Self::new(code).with_type(ContentType::Html).with_body(body)
    }

    /// # Errors
    /// Returns an error when it fails to serialize `v`.
    #[cfg(feature = "json")]
    pub fn json(code: u16, v: impl serde::Serialize) -> Result<Response, String> {
        let body_vec = serde_json::to_vec(&v)
            .map_err(|e| format!("error serializing response to json: {}", e))?;
        Ok(Self::new(code)
            .with_type(ContentType::Json)
            .with_body(body_vec))
    }

    #[must_use]
    pub fn text(code: u16, body: impl Into<ResponseBody>) -> Self {
        Self::new(code)
            .with_type(ContentType::PlainText)
            .with_body(body)
    }

    /// Tell the client to GET `location`.
    ///
    /// The client should not store this redirect.
    ///
    /// A PUT or POST handler usually returns this.
    ///
    /// # Panics
    /// Panics when `location` is not US-ASCII.
    #[must_use]
    pub fn redirect_303(location: impl AsRef<str>) -> Self {
        Response::new(303).with_header("location", location.as_ref().try_into().unwrap())
    }

    /// # Panics
    /// Panics when any of `allowed_methods` are not US-ASCII.
    #[must_use]
    pub fn method_not_allowed_405(allowed_methods: &[&'static str]) -> Self {
        Self::new(405).with_header("allow", allowed_methods.join(",").try_into().unwrap())
    }

    #[must_use]
    pub fn length_required_411() -> Self {
        Response::text(411, "not accepting streaming uploads")
    }

    #[must_use]
    pub fn payload_too_large_413() -> Self {
        Response::text(413, "Uploaded data is too big.")
    }

    // TODO: Move this code to a Headers struct and share it with Request.

    /// Adds a header.
    ///
    /// You can call this multiple times to add multiple headers with the same name.
    ///
    /// The [HTTP spec](https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.4)
    /// limits header names to US-ASCII and header values to US-ASCII or ISO-8859-1.
    ///
    /// # Panics
    /// Panics when `name` is not US-ASCII.
    pub fn add_header(&mut self, name: impl AsRef<str>, value: AsciiString) {
        self.headers
            .push((name.as_ref().try_into().unwrap(), value));
    }

    /// Finds the first header that matches `name` with a case-insensitive comparison and
    /// returns its `value`.
    ///
    /// Returns `None` if the no name matched.
    pub fn get_first_header(&self, name: impl AsRef<str>) -> Option<&AsciiString> {
        for (header_name, header_value) in &self.headers {
            if header_name.eq_ignore_ascii_case(name.as_ref()) {
                return Some(header_value);
            }
        }
        None
    }

    /// Looks for headers with names that match `name` with a case-insensitive comparison.
    /// Returns the values of those headers.
    pub fn get_headers(&self, name: impl AsRef<str>) -> Vec<&AsciiString> {
        let mut headers = Vec::new();
        for (header_name, header_value) in &self.headers {
            if header_name.eq_ignore_ascii_case(name.as_ref()) {
                headers.push(header_value);
            }
        }
        headers
    }

    /// Removes the first header that matches `name` with a case-insensitive comparison and
    /// has `value`.
    ///
    /// Returns `None` if the no name matched.
    pub fn remove_header(
        &mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Option<(AsciiString, AsciiString)> {
        for (n, (header_name, header_value)) in self.headers.iter().enumerate() {
            if header_name.eq_ignore_ascii_case(name.as_ref())
                && header_value.as_str() == value.as_ref()
            {
                return Some(self.headers.remove(n));
            }
        }
        None
    }

    #[must_use]
    pub fn with_body(mut self, b: impl Into<ResponseBody>) -> Self {
        self.body = b.into();
        self
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
    ///
    /// # Example
    /// ```
    /// use beatrice::{AsciiString, Response};
    /// use core::convert::TryInto;
    /// # fn new_random_session_id_u64() -> u64 { 123 }
    ///
    /// let session_id: u64 = new_random_session_id_u64();
    /// // ...
    /// return Response::redirect_303("/logged-in")
    ///     .with_cookie("session_id", session_id.into())
    ///     .with_cookie("backend", "prod0".to_string().try_into().unwrap());
    /// ```
    #[must_use]
    pub fn with_header(mut self, name: impl AsRef<str>, value: AsciiString) -> Self {
        self.add_header(name, value);
        self
    }

    #[must_use]
    pub fn with_status(mut self, c: u16) -> Self {
        self.code = c;
        self
    }

    #[must_use]
    pub fn with_type(mut self, t: ContentType) -> Self {
        self.content_type = t;
        self
    }

    #[must_use]
    pub fn is_1xx(&self) -> bool {
        self.code / 100 == 1
    }

    #[must_use]
    pub fn is_2xx(&self) -> bool {
        self.code / 100 == 2
    }

    #[must_use]
    pub fn is_3xx(&self) -> bool {
        self.code / 100 == 3
    }

    #[must_use]
    pub fn is_4xx(&self) -> bool {
        self.code / 100 == 4
    }

    #[must_use]
    pub fn is_5xx(&self) -> bool {
        self.code / 100 == 5
    }

    #[must_use]
    pub fn is_normal(&self) -> bool {
        self.kind == ResponseKind::Normal
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
        match self.kind {
            ResponseKind::DropConnection => write!(f, "Response(kind=Drop)"),
            ResponseKind::GetBodyAndReprocess(max_len) => {
                write!(f, "Response(kind=GetBodyAndReprocess({}))", max_len)
            }
            ResponseKind::Normal => {
                let mut headers: Vec<String> = self
                    .headers
                    .iter()
                    .map(|(n, v)| format!("{}: {:?}", n, v))
                    .collect();
                headers.sort();
                write!(
                    f,
                    "Response({} {}, {:?}, headers={{{}}}, {:?})",
                    self.code,
                    reason_phrase(self.code),
                    self.content_type,
                    self.headers
                        .iter()
                        .map(|(name, value)| format!("{}:{}", name, value))
                        .collect::<Vec<String>>()
                        .join(", "),
                    self.body
                )
            }
        }
    }
}

#[must_use]
pub fn reason_phrase(code: u16) -> &'static str {
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
    match code {
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
        response.code,
        reason_phrase(response.code)
    )
    .into_bytes();
    if response.content_type != ContentType::None {
        for (name, _value) in &response.headers {
            if name.as_str().eq_ignore_ascii_case("content-type") {
                return Err(HttpError::DuplicateContentTypeHeader);
            }
        }
        write!(
            head_bytes,
            "content-type: {}\r\n",
            response.content_type.as_str()
        )
        .unwrap();
    }
    let body_len = response.body.len();
    for (name, _value) in &response.headers {
        if name.as_str().eq_ignore_ascii_case("content-length") {
            return Err(HttpError::DuplicateContentLengthHeader);
        }
    }
    write!(head_bytes, "content-length: {}\r\n", body_len).unwrap();
    for (name, value) in &response.headers {
        // Convert headers from UTF-8 back to ISO-8859-1, with 0xFF for a replacement byte.
        write!(head_bytes, "{}: ", name).unwrap();
        head_bytes.extend(value.chars().map(|c| u8::try_from(c).unwrap_or(255)));
        head_bytes.extend(b"\r\n");
    }
    head_bytes.extend(b"\r\n");
    //dbg!(escape_ascii(head_bytes.as_slice()));
    writer
        .write_all(head_bytes.as_slice())
        .await
        .map_err(|_| HttpError::Disconnected)?;
    drop(head_bytes);
    if body_len > 0 {
        //dbg!(body_len);
        let mut reader = AsyncReadExt::take(
            response
                .body
                .async_reader()
                .await
                .map_err(HttpError::error_reading_file)?,
            body_len,
        );
        match copy_async(&mut reader, &mut writer).await {
            CopyResult::Ok(len) if len == body_len => {
                //dbg!(len);
            }
            CopyResult::Ok(_len) => {
                return Err(HttpError::ErrorReadingFile(
                    ErrorKind::UnexpectedEof,
                    "body file is smaller than expected".to_string(),
                ))
            }
            CopyResult::ReaderErr(e) => return Err(HttpError::error_reading_file(e)),
            CopyResult::WriterErr(..) => return Err(HttpError::Disconnected),
        };
    }
    let result = writer.flush().await.map_err(|_| HttpError::Disconnected);
    //dbg!(&result);
    result
}
