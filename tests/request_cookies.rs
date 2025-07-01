// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cookie
use crate::test_util::{TestServer, assert_ends_with, assert_starts_with};
use servlin::Response;

mod test_util;

#[test]
fn cookie_header() {
    let server = TestServer::start(|req| {
        let mut strings: Vec<String> = req
            .cookies
            .iter()
            .map(|(name, value)| format!("({name}:{value})"))
            .collect();
        strings.sort();
        Response::text(200, format!("cookies=[{}]", strings.join(",")))
    })
    .unwrap();
    assert_ends_with(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "cookies=[]",
    );
    assert_ends_with(
        server.exchange("M / HTTP/1.1\r\ncookie:\r\n\r\n").unwrap(),
        "cookies=[]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\nCookie: a=b\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b;\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: ;a=b\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b; c=d\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b),(c:d)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b;; c=d\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b),(c:d)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b; ;c=d\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b),(c:d)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b; c=d;\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b),(c:d)]",
    );
    assert_ends_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b;c=d;e=123 4=5\r\n\r\n")
            .unwrap(),
        "cookies=[(a:b),(c:d),(e:123 4=5)]",
    );
    // Malformed cookie header
    assert_starts_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie:abc\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 ",
    );
    assert_starts_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b; c\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 ",
    );
    assert_starts_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a; b=c\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 ",
    );
    assert_starts_with(
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=b;c;d=e\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 ",
    );
    assert_starts_with(
        // Duplicate cookie
        server
            .exchange("M / HTTP/1.1\r\ncookie: a=1; a2;\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 ",
    );
}
