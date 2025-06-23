use crate::http_error::HttpError;
use crate::util::find_slice;
use crate::{AsciiString, Header, HeaderList};
use fixed_buffer::FixedBuf;
use futures_io::AsyncRead;
use futures_lite::AsyncReadExt;
use safe_regex::{regex, Matcher2, Matcher3};
use url::Url;

fn trim_trailing_cr(bytes: &[u8]) -> &[u8] {
    if let Some(&b'\r') = bytes.last() {
        bytes.split_last().unwrap().1
    } else {
        bytes
    }
}

fn trim_whitespace(mut bytes: &[u8]) -> &[u8] {
    loop {
        if let Some(&byte) = bytes.first() {
            if byte == b' ' || byte == b'\t' || byte == b'\r' || byte == b'\n' {
                bytes = bytes.split_first().unwrap().1;
                continue;
            }
        }
        if let Some(&byte) = bytes.last() {
            if byte == b' ' || byte == b'\t' || byte == b'\r' || byte == b'\n' {
                bytes = bytes.split_last().unwrap().1;
                continue;
            }
        }
        break;
    }
    bytes
}

#[allow(clippy::module_name_repetitions)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum HeadError {
    Truncated,
    MissingRequestLine,
    MalformedRequestLine,
    MalformedPath,
    UnsupportedProtocol,
    MalformedHeader,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Head {
    pub method: String,
    pub url: Url,
    pub headers: HeaderList,
}
impl Head {
    fn read_head_bytes<const BUF_SIZE: usize>(
        buf: &mut FixedBuf<BUF_SIZE>,
    ) -> Result<&[u8], HeadError> {
        let head_len = find_slice(b"\r\n\r\n", buf.readable()).ok_or(HeadError::Truncated)?;
        let head_bytes_with_delim = buf.try_read_exact(head_len + 4).unwrap();
        let head_bytes = &head_bytes_with_delim[0..head_len];
        Ok(head_bytes)
    }

    fn parse_request_line(line: &[u8]) -> Result<(String, Url), HeadError> {
        // https://datatracker.ietf.org/doc/html/rfc7230#section-3.1.1
        // https://datatracker.ietf.org/doc/html/rfc7230#section-5.3
        //     request-line   = method SP request-target SP HTTP-version CRLF
        //     method         = token
        //     request-target = origin-form
        //                    / absolute-form
        //                    / authority-form
        //                    / asterisk-form
        //     origin-form    = absolute-path [ "?" query ]
        //     token          = 1*tchar
        //     tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
        //                      / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
        //                      / DIGIT / ALPHA
        //                      ; any VCHAR, except delimiters
        #[allow(clippy::assign_op_pattern)]
        #[allow(clippy::range_plus_one)]
        let matcher: Matcher3<_> =
            regex!(br"([-!#$%&'*+.^_`|~0-9A-Za-z]+) ([^ \t\r\n]+) ([^ \t\r\n]+)");
        let (method_bytes, path_bytes, proto_bytes) = matcher
            .match_slices(line)
            .ok_or(HeadError::MalformedRequestLine)?;
        let method = std::str::from_utf8(method_bytes).unwrap().to_string();
        let url_string = std::str::from_utf8(path_bytes).map_err(|_| HeadError::MalformedPath)?;
        if url_string != "*" && !url_string.starts_with('/') {
            return Err(HeadError::MalformedPath);
        }
        let url = Url::options()
            .base_url(Some(&Url::parse("http://unknown/").unwrap()))
            .parse(url_string)
            .map_err(|_| HeadError::MalformedPath)?;
        if proto_bytes != b"HTTP/1.1" {
            return Err(HeadError::UnsupportedProtocol);
        }
        Ok((method, url))
    }

    fn latin1_bytes_to_utf8(bytes: &[u8]) -> String {
        bytes.iter().map(|&b| b as char).collect()
    }

    fn parse_header_line(line: &[u8]) -> Result<Header, HeadError> {
        // https://datatracker.ietf.org/doc/html/rfc7230#section-3.2
        //     header-field   = field-name ":" OWS field-value OWS
        //     field-name     = token
        //     field-value    = *( field-content )
        //     field-content  = field-vchar [ 1*( SP / HTAB ) field-vchar ]
        //     field-vchar    = VCHAR
        //     OWS            = *( SP / HTAB )
        //     token          = 1*tchar
        //     tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
        //                      / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
        //                      / DIGIT / ALPHA
        //                      ; any VCHAR, except delimiters
        //
        // "Historically, HTTP has allowed field content with text in the
        //  ISO-8859-1 charset [ISO-8859-1], supporting other charsets only
        //  through use of [RFC2047] encoding.  In practice, most HTTP header
        //  field values use only a subset of the US-ASCII charset [USASCII].
        //  Newly defined header fields SHOULD limit their field values to
        //  US-ASCII octets.  A recipient SHOULD treat other octets in field
        //  content (obs-text) as opaque data."
        #[allow(clippy::range_plus_one)]
        #[allow(clippy::assign_op_pattern)]
        let matcher: Matcher2<_> = regex!(br"([-!#$%&'*+.^_`|~0-9A-Za-z]+):[ \t]*(.*)[ \t]*");
        let (name_bytes, value_bytes) = matcher
            .match_slices(line)
            .ok_or(HeadError::MalformedHeader)?;
        let name_string = String::from_utf8(name_bytes.to_vec()).unwrap();
        let value_string = Self::latin1_bytes_to_utf8(trim_whitespace(value_bytes));
        let name = AsciiString::try_from(name_string).unwrap();
        let value = AsciiString::try_from(value_string).unwrap();
        Ok(Header::new(name, value))
    }

    /// # Errors
    /// Returns an error when:
    /// - the buffer does not contain a full request head, ending in `"\r\n"`
    /// - we fail to parse the request head
    pub fn try_read<const BUF_SIZE: usize>(
        buf: &mut FixedBuf<BUF_SIZE>,
    ) -> Result<Self, HeadError> {
        let head = Self::read_head_bytes(buf)?;
        let mut lines = head.split(|b| *b == b'\n').map(trim_trailing_cr);
        let request_line = lines.next().ok_or(HeadError::MissingRequestLine)?;
        let (method, url) = Self::parse_request_line(request_line)?;
        let mut headers = HeaderList::new();
        for line in lines {
            let header = Self::parse_header_line(line)?;
            headers.push(header);
        }
        Ok(Self {
            method,
            url,
            headers,
        })
    }
}
impl core::fmt::Debug for Head {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "Head{{method={:?}, path={:?}, query={:?}, headers={:?}}}",
            self.method,
            self.url.path(),
            self.url.query().unwrap_or(""),
            self.headers
        )
    }
}

/// # Errors
/// Returns an error when:
/// - the connection is closed
/// - we fail to read a request head
/// - the request head is too long
/// - we fail to parse the request head
#[allow(clippy::module_name_repetitions)]
pub async fn read_http_head<const BUF_SIZE: usize>(
    buf: &mut FixedBuf<BUF_SIZE>,
    mut stream: impl AsyncRead + Unpin,
) -> Result<Head, HttpError> {
    loop {
        //dbg!(&buf);
        match Head::try_read(buf) {
            Ok(head) => return Ok(head),
            Err(HeadError::Truncated) => {}
            Err(e) => return Err(e.into()),
        }
        if buf.writable().is_empty() {
            return Err(HttpError::HeadTooLong);
        }
        match stream.read(buf.writable()).await {
            Err(..) | Ok(0) if buf.is_empty() => return Err(HttpError::Disconnected),
            Err(..) | Ok(0) => return Err(HttpError::Truncated),
            Ok(n) => buf.wrote(n),
        }
    }
}
