use futures_io::AsyncWrite;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::io::Write;

use crate::event::EventReceiver;
use crate::http_error::HttpError;
use crate::util::{copy_async, copy_chunked_async};
use crate::{AsciiString, ContentType, Cookie, Error, EventSender, HeaderList, ResponseBody};
use safina_sync::sync_channel;
use std::fmt::Debug;
use std::sync::Mutex;

#[allow(clippy::module_name_repetitions)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResponseKind {
    DropConnection,
    /// `GetBodyAndReprocess(max_len: u64)`<br>
    /// Read the body from the client, but only up to the specified `u64` bytes.
    GetBodyAndReprocess(u64),
    Normal,
}

#[derive(Eq, PartialEq)]
pub struct Response {
    pub kind: ResponseKind,
    pub code: u16,
    pub content_type: ContentType,
    pub headers: HeaderList,
    pub body: ResponseBody,
}
impl Response {
    #[must_use]
    pub fn new(code: u16) -> Self {
        Self {
            kind: ResponseKind::Normal,
            code,
            content_type: ContentType::None,
            headers: HeaderList::new(),
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
            headers: HeaderList::new(),
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
            headers: HeaderList::new(),
            body: ResponseBody::empty(),
        }
    }

    /// Looks for the requested file in included `dir`.
    ///
    /// Determines the content-type from the file extension.
    /// For the list of supported content types, see [`ContentType`].
    ///
    /// When the request path is `"/"`, tries to return the file `/index.html`.
    ///
    /// # Errors
    /// Returns a 404 Not Found response if the file is not found in the included dir.
    #[cfg(feature = "include_dir")]
    // TODO: Change this to accept only GET and HEAD requests.
    // TODO: Change this to handle HEAD requests properly.
    pub fn include_dir(path: &str, dir: &'static include_dir::Dir) -> Result<Response, Error> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let path = if path.is_empty() { "index.html" } else { path };
        let file = dir
            .get_file(path)
            .ok_or_else(|| Error::client_error(Response::not_found_404()))?;
        let extension = std::path::Path::new(path)
            .extension()
            .map_or("", |os_str| os_str.to_str().unwrap_or(""));
        let content_type = match extension {
            "css" => ContentType::Css,
            "csv" => ContentType::Csv,
            "gif" => ContentType::Gif,
            "htm" | "html" => ContentType::Html,
            "js" => ContentType::JavaScript,
            "jpg" | "jpeg" => ContentType::Jpeg,
            "json" => ContentType::Json,
            "md" => ContentType::Markdown,
            "pdf" => ContentType::Pdf,
            "txt" => ContentType::PlainText,
            "png" => ContentType::Png,
            "svg" => ContentType::Svg,
            _ => ContentType::None,
        };
        return Ok(Response::new(200)
            .with_type(content_type)
            .with_body(ResponseBody::StaticBytes(file.contents())));
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
            .map_err(|e| format!("error serializing response to json: {e}"))?;
        Ok(Self::new(code)
            .with_type(ContentType::Json)
            .with_body(body_vec))
    }

    #[must_use]
    pub fn event_stream() -> (EventSender, Response) {
        let (sender, receiver) = sync_channel(50);
        (
            EventSender(Some(sender)),
            Self::new(200)
                .with_type(ContentType::EventStream)
                .with_body(ResponseBody::EventStream(Mutex::new(EventReceiver(
                    receiver,
                )))),
        )
    }

    #[must_use]
    pub fn text(code: u16, body: impl Into<ResponseBody>) -> Self {
        Self::new(code)
            .with_type(ContentType::PlainText)
            .with_body(body)
    }

    #[must_use]
    pub fn ok_200() -> Self {
        Response::new(200)
    }

    #[must_use]
    pub fn no_content_204() -> Self {
        Response::new(204)
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

    #[must_use]
    pub fn unauthorized_401() -> Self {
        Response::new(401)
    }

    #[must_use]
    pub fn forbidden_403() -> Self {
        Response::new(401)
    }

    #[must_use]
    pub fn not_found_404() -> Self {
        Response::text(404, "not found")
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
    pub fn internal_server_errror_500() -> Self {
        Response::new(500)
    }

    #[must_use]
    pub fn not_implemented_501() -> Self {
        Response::new(501)
    }

    #[must_use]
    pub fn service_unavailable_503() -> Self {
        Response::new(503)
    }

    #[must_use]
    pub fn with_body(mut self, b: impl Into<ResponseBody>) -> Self {
        self.body = b.into();
        self
    }

    /// Adds a `Cache-Control: max-age=N` header.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control>
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn with_max_age_seconds(mut self, seconds: u32) -> Self {
        self.headers.add(
            "cache-control",
            format!("max-age={seconds}").try_into().unwrap(),
        );
        self
    }

    /// Adds a `Cache-Control: no-store` header.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control>
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn with_no_store(mut self) -> Self {
        self.headers
            .add("cache-control", "no-store".try_into().unwrap());
        self
    }

    /// Adds a `Set-Cookie` header.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie>
    #[must_use]
    pub fn with_set_cookie(mut self, cookie: Cookie) -> Self {
        self.headers.add("set-cookie", cookie.into());
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
    /// use servlin::Response;
    ///
    /// # fn example() -> Response {
    /// return Response::new(200)
    ///     .with_header("header1", "value1".to_string().try_into().unwrap());
    /// # }
    /// ```
    #[must_use]
    pub fn with_header(mut self, name: impl AsRef<str>, value: AsciiString) -> Self {
        self.headers.add(name, value);
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

    #[must_use]
    pub fn is_get_body_and_reprocess(&self) -> bool {
        matches!(self.kind, ResponseKind::GetBodyAndReprocess(..))
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
                write!(f, "Response(kind=GetBodyAndReprocess({max_len}))")
            }
            ResponseKind::Normal => {
                write!(
                    f,
                    "Response({} {}, {:?}, {:?}, {:?})",
                    self.code,
                    reason_phrase(self.code),
                    self.content_type,
                    self.headers,
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
        if response.headers.get_only("content-type").is_some() {
            return Err(HttpError::DuplicateContentTypeHeader);
        }
        write!(
            head_bytes,
            "content-type: {}\r\n",
            response.content_type.as_str()
        )
        .unwrap();
    }
    if let Some(body_len) = response.body.len() {
        if response.headers.get_only("content-length").is_some() {
            return Err(HttpError::DuplicateContentLengthHeader);
        }
        write!(head_bytes, "content-length: {body_len}\r\n").unwrap();
    } else {
        if response.headers.get_only("transfer-encoding").is_some() {
            return Err(HttpError::DuplicateTransferEncodingHeader);
        }
        write!(head_bytes, "transfer-encoding: chunked\r\n").unwrap();
    }
    for header in &response.headers {
        // Convert headers from UTF-8 back to ISO-8859-1, with 0xFF for a replacement byte.
        write!(head_bytes, "{}: ", header.name).unwrap();
        head_bytes.extend(header.value.chars().map(|c| u8::try_from(c).unwrap_or(255)));
        head_bytes.extend(b"\r\n");
    }
    head_bytes.extend(b"\r\n");
    //dbg!(escape_ascii(head_bytes.as_slice()));
    writer
        .write_all(head_bytes.as_slice())
        .await
        .map_err(|_| HttpError::Disconnected)?;
    drop(head_bytes);
    match response.body.len() {
        Some(0) => {}
        Some(body_len) => {
            let mut reader = AsyncReadExt::take(
                response
                    .body
                    .async_reader()
                    .await
                    .map_err(HttpError::error_reading_file)?,
                body_len,
            );
            let num_copied = copy_async(&mut reader, &mut writer)
                .await
                .map_errs(HttpError::error_reading_response_body, |_| {
                    HttpError::Disconnected
                })?;
            if num_copied != body_len {
                return Err(HttpError::ErrorReadingResponseBody(
                    ErrorKind::UnexpectedEof,
                    "body is smaller than expected".to_string(),
                ));
            }
        }
        None => {
            let mut reader = response
                .body
                .async_reader()
                .await
                .map_err(HttpError::error_reading_response_body)?;
            copy_chunked_async(&mut reader, &mut writer)
                .await
                .map_errs(HttpError::error_reading_response_body, |_| {
                    HttpError::Disconnected
                })?;
        }
    }
    writer.flush().await.map_err(|_| HttpError::Disconnected)
}
