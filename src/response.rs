use futures_io::AsyncWrite;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::io::Write;

use crate::http_error::HttpError;
use crate::util::{copy_async, CopyResult};
use crate::{AsciiString, ContentType, Request, ResponseBody};
use std::fmt::Debug;

#[derive(Clone, Eq, PartialEq)]
pub enum Response {
    Drop,
    /// `GetBodyAndReprocess(max_len: u64, Request)`<br>
    /// Read the body from the client, but only up to the specified `u64` bytes.
    GetBodyAndReprocess(u64, Request),
    /// Normal(code: u16, ContentType, headers: Vec<String,String>, ResponseBody)
    Normal(
        u16,
        ContentType,
        Vec<(AsciiString, AsciiString)>,
        ResponseBody,
    ),
}
impl Response {
    #[must_use]
    pub fn new(code: u16) -> Self {
        Response::Normal(code, ContentType::None, Vec::new(), ResponseBody::empty())
    }

    #[must_use]
    pub fn html(code: u16, body: impl Into<ResponseBody>) -> Self {
        Response::Normal(code, ContentType::Html, Vec::new(), body.into())
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
            Vec::new(),
            ResponseBody::Vec(body_vec),
        ))
    }

    #[must_use]
    pub fn text(code: u16, body: impl Into<ResponseBody>) -> Self {
        Response::Normal(code, ContentType::PlainText, Vec::new(), body.into())
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

    #[must_use]
    fn mut_tuple(
        &mut self,
    ) -> (
        &mut u16,
        &mut ContentType,
        &mut Vec<(AsciiString, AsciiString)>,
        &mut ResponseBody,
    ) {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(c, t, h, b) => (c, t, h, b),
        }
    }

    pub fn set_body(&mut self, b: ResponseBody) {
        *self.mut_tuple().3 = b;
    }

    pub fn set_code(&mut self, c: u16) {
        *self.mut_tuple().0 = c;
    }

    pub fn set_content_type(&mut self, t: ContentType) {
        *self.mut_tuple().1 = t;
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
    pub fn add_header(&mut self, name: impl AsRef<str>, value: AsciiString) {
        self.mut_tuple()
            .2
            .push((name.as_ref().try_into().unwrap(), value));
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
        let headers = self.mut_tuple().2;
        for (n, (header_name, header_value)) in headers.iter().enumerate() {
            if header_name.eq_ignore_ascii_case(name.as_ref())
                && header_value.as_str() == value.as_ref()
            {
                return Some(headers.remove(n));
            }
        }
        None
    }

    #[must_use]
    pub fn with_body(mut self, b: impl Into<ResponseBody>) -> Self {
        self.set_body(b.into());
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
        self.set_code(c);
        self
    }

    #[must_use]
    pub fn with_type(mut self, t: ContentType) -> Self {
        self.set_content_type(t);
        self
    }

    #[must_use]
    pub fn is_normal(&self) -> bool {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => false,
            Response::Normal(..) => true,
        }
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn body(&self) -> &ResponseBody {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, _t, _h, b) => b,
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

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn content_type(&self) -> &ContentType {
        match self {
            // TODO: Remove these panics.  Restructure somehow.
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, t, _h, _b) => t,
        }
    }

    /// # Panics
    /// Panics when called on `Response::Drop` or `Response::GetBodyAndReprocess(..)`.
    #[must_use]
    pub fn headers(&self) -> &Vec<(AsciiString, AsciiString)> {
        match self {
            Response::Drop | Response::GetBodyAndReprocess(..) => unimplemented!(),
            Response::Normal(_c, _t, h, _b) => h,
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
                    reason_phrase(self.code()),
                    self.content_type(),
                    headers.join(", "),
                    body
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
        response.code(),
        reason_phrase(response.code())
    )
    .into_bytes();
    if response.content_type() != &ContentType::None {
        for (name, _value) in response.headers() {
            if name.as_str().eq_ignore_ascii_case("content-type") {
                return Err(HttpError::DuplicateContentTypeHeader);
            }
        }
        write!(
            head_bytes,
            "content-type: {}\r\n",
            response.content_type().as_str()
        )
        .unwrap();
    }
    let body_len = response.body().len();
    for (name, _value) in response.headers() {
        if name.as_str().eq_ignore_ascii_case("content-length") {
            return Err(HttpError::DuplicateContentLengthHeader);
        }
    }
    write!(head_bytes, "content-length: {}\r\n", body_len).unwrap();
    for (name, value) in response.headers() {
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
                .body()
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
