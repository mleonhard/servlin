use safe_regex::{Matcher9, regex};
use std::net::IpAddr;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UrlParseError {
    MalformedUrl,
    PortOutOfRange,
    InvalidIpAddress,
    UnknownIpVersion,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Url {
    pub scheme: String,
    pub user: String,
    pub host: String,
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub path: String,
    pub query: String,
    pub fragment: String,
}
impl Url {
    /// # Errors
    /// Returns an error when it fails to parse `url_s`.
    #[allow(clippy::missing_panics_doc)]
    pub fn parse_absolute(url_s: impl AsRef<[u8]>) -> Result<Self, UrlParseError> {
        // https://datatracker.ietf.org/doc/html/rfc3986
        // https://datatracker.ietf.org/doc/html/rfc7230#section-2.7
        // URI           = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
        // hier-part     = "//" authority path-abempty
        //               / path-absolute
        //               / path-rootless
        //               / path-empty
        // authority     = [ userinfo "@" ] host [ ":" port ]
        // userinfo      = *( unreserved / pct-encoded / sub-delims / ":" )
        // unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
        // pct-encoded   = "%" HEXDIG HEXDIG
        // sub-delims    = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
        // host          = IP-literal / IPv4address / reg-name
        // IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
        // IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
        // IPv6address   =                            6( h16 ":" ) ls32
        //               /                       "::" 5( h16 ":" ) ls32
        //               / [               h16 ] "::" 4( h16 ":" ) ls32
        //               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
        //               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
        //               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
        //               / [ *4( h16 ":" ) h16 ] "::"              ls32
        //               / [ *5( h16 ":" ) h16 ] "::"              h16
        //               / [ *6( h16 ":" ) h16 ] "::"
        //       ls32    = ( h16 ":" h16 ) / IPv4address
        //               ; least-significant 32 bits of address
        //       h16     = 1*4HEXDIG
        //               ; 16 bits of address represented in hexadecimal
        // IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
        // reg-name      = *( unreserved / pct-encoded / sub-delims )
        // port          = *DIGIT
        // path-abempty  = *( "/" segment )
        // path-absolute = "/" [ segment-nz *( "/" segment ) ]
        // path-noscheme = segment-nz-nc *( "/" segment )
        // path-rootless = segment-nz *( "/" segment )
        // path-empty    = 0<pchar>
        // segment       = *pchar
        // segment-nz    = 1*pchar
        // segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
        //               ; non-zero-length segment without any colon ":"
        // pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
        // query         = *( pchar / "/" / "?" )
        // fragment      = *( pchar / "/" / "?" )
        let orig_bytes = url_s.as_ref();
        let matcher: Matcher9<_> = regex!(br"([-.+0-9A-Za-z]+)://(?:([-._~a-zA-Z0-9%!$&'()*,;=:]*)@)?(?:([0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3})|(\[[-._~a-zA-Z0-9%!$&'()*,;=:]+])|([-._~a-zA-Z0-9%!$&'()*,;=]+))(?::([0-9]*))?(/[-._~a-zA-Z0-9%!$&'()*,;=:@/]*)?(?:\?([-._~a-zA-Z0-9%!$&'()*,;=:@/?]*))?(?:#([-._~a-zA-Z0-9%!$&'()*,;=:@/?]*))?");
        let (
            scheme_bytes,
            user_bytes,
            ipv4_bytes,
            ipv6_bytes,
            host_bytes,
            port_bytes,
            path_bytes,
            query_bytes,
            fragment_bytes,
        ) = matcher
            .match_slices(orig_bytes)
            .ok_or(UrlParseError::MalformedUrl)?;
        let scheme = std::str::from_utf8(scheme_bytes).unwrap().to_string();
        let user = std::str::from_utf8(user_bytes).unwrap().to_string();
        let ip: Option<IpAddr> = if !ipv4_bytes.is_empty() {
            Some(
                std::str::from_utf8(ipv4_bytes)
                    .unwrap()
                    .parse::<IpAddr>()
                    .map_err(|_| UrlParseError::InvalidIpAddress)?,
            )
        } else if !ipv6_bytes.is_empty() {
            let b = &ipv6_bytes[1..(ipv6_bytes.len() - 1)];
            if b[0] == b'v' {
                return Err(UrlParseError::UnknownIpVersion);
            }
            Some(
                std::str::from_utf8(b)
                    .unwrap()
                    .parse::<IpAddr>()
                    .map_err(|_| UrlParseError::InvalidIpAddress)?,
            )
        } else {
            None
        };
        let host = std::str::from_utf8(host_bytes).unwrap().to_string();
        let port: Option<u16> = match port_bytes.len() {
            0 => None,
            1..6 => Some(
                std::str::from_utf8(port_bytes)
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
                    .try_into()
                    .map_err(|_| UrlParseError::PortOutOfRange)?,
            ),
            _ => return Err(UrlParseError::PortOutOfRange),
        };
        let path = std::str::from_utf8(path_bytes).unwrap().to_string();
        let query = std::str::from_utf8(query_bytes).unwrap().to_string();
        let fragment = std::str::from_utf8(fragment_bytes).unwrap().to_string();
        Ok(Self {
            scheme,
            user,
            host,
            ip,
            port,
            path,
            query,
            fragment,
        })
    }
}
