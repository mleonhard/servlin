#![cfg(feature = "internals")]

use crate::test_util::{assert_ends_with, assert_starts_with, check_elapsed, TestServer};
use beatrice::{ContentType, Response};
use serde_json::json;
use std::io::Read;
use std::time::{Duration, Instant};

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
fn empty() {
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
fn with_status() {
    let server = TestServer::start(|_req| Response::new(200).with_status(201)).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 201 Created\r\n\r\n",
    );
}

#[test]
fn with_type() {
    let server =
        TestServer::start(|_req| Response::new(200).with_type(ContentType::EventStream)).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n",
    );
}

#[test]
fn with_type_and_body() {
    let server =
        TestServer::start(|_req| Response::text(200, "yo").with_type(ContentType::Markdown))
            .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/markdown; charset=UTF-8\r\ncontent-length: 2\r\n\r\nyo",
    );
}

#[test]
fn with_body() {
    let server = TestServer::start(|_req| Response::new(200).with_body("abc")).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\nabc",
    );
}

#[test]
fn with_header() {
    let server = TestServer::start(|_req| Response::new(200).with_header("h1", "v1")).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\nh1: v1\r\n\r\n",
    );
}

#[test]
fn with_duplicate_header() {
    let server = TestServer::start(|_req| {
        Response::new(200)
            .with_header("h1", "v1")
            .with_header("h1", "v2")
    })
    .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\nh1: v2\r\n\r\n",
    );
}

#[test]
fn with_duplicate_header_different_case() {
    let server = TestServer::start(|_req| {
        Response::new(200)
            .with_header("h1", "v1")
            .with_header("H1", "v2")
    })
    .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\nh1: v2\r\n\r\n",
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

#[test]
fn get_body() {
    let server = TestServer::start(|req| {
        if req.body().is_pending() {
            return Response::GetBodyAndReprocess(70_000, req);
        } else {
            let len = req.body().reader().unwrap().bytes().count();
            Response::text(200, format!("len={}", len))
        }
    })
    .unwrap();
    fn req_with_len(body_len: usize) -> String {
        format!("M / HTTP/1.1\r\ncontent-length:{}\r\n\r\n", body_len)
            .chars()
            .chain(std::iter::repeat('a').take(body_len))
            .collect::<String>()
    }
    assert_ends_with(server.exchange(req_with_len(0)).unwrap(), "len=0");
    assert_ends_with(server.exchange(req_with_len(1)).unwrap(), "len=1");
    assert_ends_with(server.exchange(req_with_len(65_536)).unwrap(), "len=65536");
    assert_ends_with(server.exchange(req_with_len(65_537)).unwrap(), "len=65537");
    assert_ends_with(server.exchange(req_with_len(70_000)).unwrap(), "len=70000");
    assert_eq!(
        server.exchange(req_with_len(70_001)).unwrap(),
        "HTTP/1.1 413 Payload Too Large\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 25\r\n\r\nUploaded data is too big.",
    );

    fn req_without_len(body_len: usize) -> String {
        format!("POST / HTTP/1.1\r\n\r\n")
            .chars()
            .chain(std::iter::repeat('a').take(body_len))
            .collect::<String>()
    }
    assert_ends_with(server.exchange(req_without_len(0)).unwrap(), "len=0");
    assert_ends_with(server.exchange(req_without_len(1)).unwrap(), "len=1");
    assert_ends_with(
        server.exchange(req_without_len(65_536)).unwrap(),
        "len=65536",
    );
    assert_ends_with(
        server.exchange(req_without_len(65_537)).unwrap(),
        "len=65537",
    );
    assert_ends_with(
        server.exchange(req_without_len(70_000)).unwrap(),
        "len=70000",
    );
    assert_eq!(
        server.exchange(req_without_len(70_001)).unwrap(),
        "HTTP/1.1 413 Payload Too Large\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 25\r\n\r\nUploaded data is too big.",
    );
}

#[test]
fn body_not_pending() {
    let server = TestServer::start(|req| Response::GetBodyAndReprocess(100, req)).unwrap();
    assert_eq!(
        server
            .exchange("M / HTTP/1.1\r\ncontent-length:3\r\n\r\nabc")
            .unwrap(),
        "HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 21\r\n\r\nInternal server error",
    );
}

#[test]
fn already_got_body() {
    let server = TestServer::start(|req| Response::GetBodyAndReprocess(100, req)).unwrap();
    assert_eq!(
        server
            .exchange("M / HTTP/1.1\r\ncontent-length:3\r\nexpect:100-continue\r\n\r\nabc")
            .unwrap(),
        "HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 21\r\n\r\nInternal server error",
    );
}

#[test]
fn fast_reply() {
    let server = TestServer::start(|_req| Response::new(200)).unwrap();
    let before = Instant::now();
    let reply = server.exchange("M / HTTP/1.1\r\n\r\n").unwrap();
    check_elapsed(before, 0..100).unwrap();
    assert_eq!(reply, "HTTP/1.1 200 OK\r\n\r\n",);
}

#[test]
fn slow_reply() {
    let server = TestServer::start(|_req| {
        std::thread::sleep(Duration::from_millis(100));
        Response::new(200)
    })
    .unwrap();
    let before = Instant::now();
    let reply = server.exchange("M / HTTP/1.1\r\n\r\n").unwrap();
    check_elapsed(before, 100..200).unwrap();
    assert_eq!(reply, "HTTP/1.1 200 OK\r\n\r\n",);
}
