use servlin::{PercentEncodePurpose, Url, UrlParseError, percent_decode, percent_encode};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn percent_decode_test() {
    assert_eq!(percent_decode(""), "");
    assert_eq!(percent_decode("abc"), "abc");
    assert_eq!(percent_decode("%"), "%");
    assert_eq!(percent_decode("%2"), "%2");
    assert_eq!(percent_decode("%2X"), "%2X");
    assert_eq!(percent_decode("%2%2a"), "%2*");
    assert_eq!(percent_decode("%2a"), "*");
    assert_eq!(percent_decode("%2A"), "*");
    assert_eq!(percent_decode("%c3%a6"), "æ");
    assert_eq!(percent_decode("a%c3%a6b"), "aæb");
    assert_eq!(percent_decode("%c3"), "\u{fffd}");
    assert_eq!(percent_decode("%c3X"), "\u{fffd}X");
}

#[test]
fn percent_encode_test() {
    assert_eq!(percent_encode("", PercentEncodePurpose::Path), "");
    assert_eq!(percent_encode("abc", PercentEncodePurpose::Path), "abc");
    assert_eq!(percent_encode("%", PercentEncodePurpose::Path), "%25");
    assert_eq!(percent_encode("%2", PercentEncodePurpose::Path), "%252");
    assert_eq!(percent_encode("%2X", PercentEncodePurpose::Path), "%252X");
    assert_eq!(percent_encode("%2#", PercentEncodePurpose::Path), "%252%23");
    assert_eq!(percent_encode("#", PercentEncodePurpose::Path), "%23");
    assert_eq!(percent_encode("æ", PercentEncodePurpose::Path), "%C3%A6");
    assert_eq!(
        percent_encode("aæb", PercentEncodePurpose::Path),
        "a%C3%A6b"
    );
    assert_eq!(
        percent_encode("\u{fffd}", PercentEncodePurpose::Path),
        "%EF%BF%BD"
    );
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

#[test]
fn parse_relative_path() {
    assert_eq!(Url::parse_relative("").unwrap().path, "");
    assert_eq!(Url::parse_relative("/").unwrap().path, "/");
    assert_eq!(Url::parse_relative("/p1").unwrap().path, "/p1");
    assert_eq!(
        Url::parse_relative(
            "/-._~%!$&'()*,;=:@/abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%C3%A6"
        )
            .unwrap()
            .path,
        "/-._~%!$&'()*,;=:@/abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789æ"
    );
    assert_eq!(Url::parse_relative("/^"), Err(UrlParseError::MalformedUrl));
}

#[test]
fn parse_relative_query() {
    assert_eq!(Url::parse_relative("").unwrap().query, "");
    assert_eq!(Url::parse_relative("?").unwrap().query, "");
    assert_eq!(Url::parse_relative("/p1?q1").unwrap().query, "q1");
    assert_eq!(Url::parse_relative("/p1?a=b").unwrap().query, "a=b");
    assert_eq!(Url::parse_relative("/p1?a=b&c").unwrap().query, "a=b&c");
    assert_eq!(
        Url::parse_relative(
            "/p?-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        )
        .unwrap()
        .query,
        "-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    );
    assert_eq!(Url::parse_relative("/?^"), Err(UrlParseError::MalformedUrl));
}

#[test]
fn parse_relative_fragment() {
    assert_eq!(Url::parse_relative("").unwrap().fragment, "");
    assert_eq!(Url::parse_relative("#").unwrap().fragment, "");
    assert_eq!(Url::parse_relative("#f1").unwrap().fragment, "f1");
    assert_eq!(Url::parse_relative("/p1#f1").unwrap().fragment, "f1");
    assert_eq!(
        Url::parse_relative(
            "/p#-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        )
        .unwrap()
        .fragment,
        "-._~!$&'()*,;=:@/?%abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    );
    assert_eq!(Url::parse_relative("#^"), Err(UrlParseError::MalformedUrl));
}

#[test]
fn display() {
    // TODO: Test chars that are invalid in userinfo, query, fragment, etc.
    assert_eq!(
        Url::parse_absolute("http://u1:p1@h1:2/d1?q1#f1")
            .unwrap()
            .to_string(),
        "http://u1:p1@h1:2/d1?q1#f1"
    );
    assert_eq!(
        Url::parse_absolute("http://u1:p1@h1").unwrap().to_string(),
        "http://u1:p1@h1"
    );
    assert_eq!(
        Url::parse_absolute("http://h1:2").unwrap().to_string(),
        "http://h1:2"
    );
    assert_eq!(
        Url::parse_absolute("http://h1/d1").unwrap().to_string(),
        "http://h1/d1"
    );
    assert_eq!(
        Url::parse_absolute("http://h1/d1%23").unwrap().to_string(),
        "http://h1/d1%23"
    );
    assert_eq!(
        Url::parse_absolute("http://h1?q1").unwrap().to_string(),
        "http://h1?q1"
    );
    assert_eq!(
        Url::parse_absolute("http://h1#f1").unwrap().to_string(),
        "http://h1#f1"
    );

    assert_eq!(
        Url::parse_relative("/d1?q1#f1").unwrap().to_string(),
        "/d1?q1#f1"
    );
    assert_eq!(Url::parse_relative("d1").unwrap().to_string(), "d1");
    assert_eq!(Url::parse_relative("/d1").unwrap().to_string(), "/d1");
    assert_eq!(Url::parse_relative("/d1%23").unwrap().to_string(), "/d1%23");
    // Test chars outside of `path-abempty` `[-._~a-zA-Z0-9%!$&'()*,;=:@/]`.
    assert_eq!(Url::parse_relative("/d1%5e").unwrap().to_string(), "/d1%5E");
    assert_eq!(
        Url::parse_relative("/d1%00%01%02%03%04%05%06%07%08%09%0a%0b%0c%0d%0f%10%11%12%13%14%15%16%17%18%19%1a%1b%1c%1d%1f")
            .unwrap()
            .to_string(),
        "/d1%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0F%10%11%12%13%14%15%16%17%18%19%1A%1B%1C%1D%1F"
    );
    assert_eq!(
        Url::parse_relative("/d1%20%22%23%25%2b%3c%3e%3f%5c%5d%5e%60%7b%7c%7d")
            .unwrap()
            .to_string(),
        "/d1%20%22%23%25%2B%3C%3E%3F%5C%5D%5E%60%7B%7C%7D"
    );
    assert_eq!(Url::parse_relative("?q1").unwrap().to_string(), "?q1");
    assert_eq!(Url::parse_relative("#f1").unwrap().to_string(), "#f1");
}
