#![cfg(feature = "internals")]

use crate::test_util::{assert_starts_with, TestServer};
use beatrice::Response;
use serde_json::json;

mod test_util;

#[test]
fn panics() {
    let server = TestServer::start(|_req| panic!("ignore this panic")).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 12\r\n\r\nServer error",
    );
}

#[test]
fn return_empty() {
    let server = TestServer::start(|_req| Response::new(200)).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\n\r\n",
    );
}

#[test]
fn unknown_code() {
    let server = TestServer::start(|_req| Response::new(123)).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 123 Response\r\n\r\n",
    );
}

#[test]
fn json() {
    let server =
        TestServer::start(|_req| Response::json(200, json!({"key":123})).unwrap()).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=UTF-8\r\ncontent-length: 11\r\n\r\n{\"key\":123}",
    );
}

#[test]
fn text() {
    let server =
        TestServer::start(|_req| Response::text(200, "abc def\tghi\rjkl\nmno\r\npqr\r\n\r\nstu"))
            .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 31\r\n\r\nabc def\tghi\rjkl\nmno\r\npqr\r\n\r\nstu",
    );
}

#[test]
fn method_not_allowed_405() {
    let server = TestServer::start(|_req| Response::method_not_allowed_405(&["GET"])).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 405 Method Not Allowed\r\nallow: GET\r\n\r\n",
    );
}

#[test]
fn duplicate_content_type_header() {
    let server = TestServer::start(|_req| {
        Response::text(200, "t1").with_header("Content-type", "text/plain")
    })
    .unwrap();
    assert_starts_with(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 500 ",
    );
}

#[test]
fn duplicate_content_length_header() {
    let server =
        TestServer::start(|_req| Response::text(200, "t1").with_header("Content-length", "0"))
            .unwrap();
    assert_starts_with(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 500 ",
    );
}

#[test]
fn return_drop() {
    let server = TestServer::start(|_req| Response::Drop).unwrap();
    assert_eq!(server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(), "");
}
