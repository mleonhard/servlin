use servlin::{Url, UrlParseError};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn percent_decode() {
    assert_eq!(Url::percent_decode(""), "");
    assert_eq!(Url::percent_decode("abc"), "abc");
    assert_eq!(Url::percent_decode("%"), "%");
    assert_eq!(Url::percent_decode("%2"), "%2");
    assert_eq!(Url::percent_decode("%2X"), "%2X");
    assert_eq!(Url::percent_decode("%2%2a"), "%2*");
    assert_eq!(Url::percent_decode("%2a"), "*");
    assert_eq!(Url::percent_decode("%2A"), "*");
    assert_eq!(Url::percent_decode("%c3%a6"), "æ");
    assert_eq!(Url::percent_decode("a%c3%a6b"), "aæb");
    assert_eq!(Url::percent_decode("%c3"), "\u{fffd}");
    assert_eq!(Url::percent_decode("%c3X"), "\u{fffd}X");
}

#[test]
fn percent_encode() {
    assert_eq!(Url::percent_encode(""), "");
    assert_eq!(Url::percent_encode("abc"), "abc");
    assert_eq!(Url::percent_encode("%"), "%");
    assert_eq!(Url::percent_encode("%2"), "%2");
    assert_eq!(Url::percent_encode("%2X"), "%2X");
    assert_eq!(Url::percent_encode("%2*"), "%2%2A");
    assert_eq!(Url::percent_encode("*"), "%2A");
    assert_eq!(Url::percent_encode("æ"), "%C3%A6");
    assert_eq!(Url::percent_encode("aæb"), "a%C3%A6b");
    assert_eq!(Url::percent_encode("\u{fffd}"), "%EF%BF%BD");
}

#[test]
fn parse_absolute() {
    assert_eq!(Url::parse_absolute(""), Err(UrlParseError::MalformedUrl));
    assert_eq!(
        Url::parse_absolute("http://u1:p1@h1:2/d1?q1#f1"),
        Ok(Url {
            scheme: "http".to_string(),
            user: "u1:p1".to_string(),
            host: "h1".to_string(),
            ip: None,
            port: Some(2),
            path: "/d1".to_string(),
            query: "q1".to_string(),
            fragment: "f1".to_string(),
        })
    );
}

#[test]
fn parse_absolute_scheme() {
    assert_eq!(
        Url::parse_absolute("://h/"),
        Err(UrlParseError::MalformedUrl)
    );
    assert_eq!(Url::parse_absolute("http://h/").unwrap().scheme, "http");
    assert_eq!(
        Url::parse_absolute(
            "-.+abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789://h/"
        )
        .unwrap()
        .scheme,
        "-.+abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
    );
    assert_eq!(
        Url::parse_absolute("a:b://h/"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_user() {
    assert_eq!(Url::parse_absolute("http://@h/").unwrap().user, "");
    assert_eq!(Url::parse_absolute("http://u@h/").unwrap().user, "u");
    assert_eq!(Url::parse_absolute("http://u:p@h/").unwrap().user, "u:p");
    assert_eq!(
        Url::parse_absolute(
            "http://-._~!$&'()*,;=:%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@h/"
        )
        .unwrap()
        .user,
        "-._~!$&'()*,;=:%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    );
    assert_eq!(
        Url::parse_absolute("http://@@h/"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_host() {
    assert_eq!(
        Url::parse_absolute("http:///"),
        Err(UrlParseError::MalformedUrl)
    );
    assert_eq!(Url::parse_absolute("http://h/").unwrap().host, "h");
    assert_eq!(
        Url::parse_absolute("http://example.com/").unwrap().host,
        "example.com"
    );
    assert_eq!(
        Url::parse_absolute(
            "http://-._~abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%!$&'()*,;=/"
        )
        .unwrap()
        .host,
        "-._~abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%!$&'()*,;="
    );
    assert_eq!(
        Url::parse_absolute("http://h^/"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_ip() {
    assert_eq!(
        Url::parse_absolute("http:///"),
        Err(UrlParseError::MalformedUrl)
    );
    assert_eq!(Url::parse_absolute("http://h/").unwrap().ip, None);
    assert_eq!(Url::parse_absolute("http://1.2.3/").unwrap().ip, None);
    assert_eq!(Url::parse_absolute("http://1.2.3/").unwrap().host, "1.2.3");
    assert_eq!(
        Url::parse_absolute("http://1.2.3.4/").unwrap().ip,
        Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
    );
    assert_eq!(
        Url::parse_absolute("http://[::]").unwrap().ip,
        Some(IpAddr::V6(Ipv6Addr::from([0, 0, 0, 0, 0, 0, 0, 0])))
    );
    assert_eq!(
        Url::parse_absolute("http://[::1]").unwrap().ip,
        Some(IpAddr::V6(Ipv6Addr::from([0, 0, 0, 0, 0, 0, 0, 1])))
    );
    assert_eq!(
        Url::parse_absolute("http://[2001:0db8:0000:0000:0000:ff00:0042:8329]")
            .unwrap()
            .ip,
        Some(IpAddr::V6(Ipv6Addr::from([
            0x2001, 0x0db8, 0x0000, 0x0000, 0x0000, 0xff00, 0x0042, 0x8329
        ])))
    );
    assert_eq!(
        Url::parse_absolute("http://[2001:db8:0:0:0:ff00:42:8329]")
            .unwrap()
            .ip,
        Some(IpAddr::V6(Ipv6Addr::from([
            0x2001, 0x0db8, 0x0000, 0x0000, 0x0000, 0xff00, 0x0042, 0x8329
        ])))
    );
    assert_eq!(
        Url::parse_absolute("http://[2001:db8::ff00:42:8329]")
            .unwrap()
            .ip,
        Some(IpAddr::V6(Ipv6Addr::from([
            0x2001, 0x0db8, 0x0000, 0x0000, 0x0000, 0xff00, 0x0042, 0x8329
        ])))
    );
    assert_eq!(
        Url::parse_absolute("http://[v0.a:b]/"),
        Err(UrlParseError::UnknownIpVersion)
    );
    assert_eq!(
        Url::parse_absolute("http://[::x]/"),
        Err(UrlParseError::InvalidIpAddress)
    );
}

#[test]
fn parse_absolute_port() {
    assert_eq!(Url::parse_absolute("http://h:/").unwrap().port, None);
    assert_eq!(Url::parse_absolute("http://h:0/").unwrap().port, Some(0));
    assert_eq!(
        Url::parse_absolute("http://h:65535/").unwrap().port,
        Some(65535)
    );
    assert_eq!(
        Url::parse_absolute("http://h:65536/"),
        Err(UrlParseError::PortOutOfRange)
    );
    assert_eq!(
        Url::parse_absolute("http://h:9999999999999999999/"),
        Err(UrlParseError::PortOutOfRange)
    );
    assert_eq!(
        Url::parse_absolute("http://h:-10/"),
        Err(UrlParseError::MalformedUrl)
    );
    assert_eq!(
        Url::parse_absolute("http://h:^/"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_path() {
    assert_eq!(Url::parse_absolute("http://h").unwrap().path, "");
    assert_eq!(Url::parse_absolute("http://h/").unwrap().path, "/");
    assert_eq!(Url::parse_absolute("http://h/p1").unwrap().path, "/p1");
    // assert_eq!(Url::parse_absolute("http:///p1").unwrap().path, "/p1");
    assert_eq!(
        Url::parse_absolute(
            "http://h/-._~%!$&'()*,;=:@/abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%C3%A6"
        )
            .unwrap()
            .path,
        "/-._~%!$&'()*,;=:@/abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789æ"
    );
    assert_eq!(
        Url::parse_absolute("http://h/^"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_query() {
    assert_eq!(Url::parse_absolute("http://h").unwrap().query, "");
    assert_eq!(Url::parse_absolute("http://h?").unwrap().query, "");
    assert_eq!(Url::parse_absolute("http://h/p1?q1").unwrap().query, "q1");
    assert_eq!(Url::parse_absolute("http://h/p1?a=b").unwrap().query, "a=b");
    assert_eq!(
        Url::parse_absolute("http://h/p1?a=b&c").unwrap().query,
        "a=b&c"
    );
    assert_eq!(
        Url::parse_absolute(
            "http://h/p?-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        )
            .unwrap()
            .query,
        "-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    );
    assert_eq!(
        Url::parse_absolute("http://h/?^"),
        Err(UrlParseError::MalformedUrl)
    );
}

#[test]
fn parse_absolute_fragment() {
    assert_eq!(Url::parse_absolute("http://h").unwrap().fragment, "");
    assert_eq!(Url::parse_absolute("http://h#").unwrap().fragment, "");
    assert_eq!(Url::parse_absolute("http://h#f1").unwrap().fragment, "f1");
    assert_eq!(
        Url::parse_absolute("http://h/p1#f1").unwrap().fragment,
        "f1"
    );
    assert_eq!(
        Url::parse_absolute(
            "http://h/p#-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        )
            .unwrap()
            .fragment,
        "-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    );
    assert_eq!(
        Url::parse_absolute("http://h#^"),
        Err(UrlParseError::MalformedUrl)
    );
}
