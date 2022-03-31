// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cookie
use crate::test_util::{assert_starts_with, TestServer};
use beatrice::{AsciiString, Cookie, Response};

mod test_util;

fn value1() -> AsciiString {
    "value1".try_into().unwrap()
}

fn value2() -> AsciiString {
    "value2".try_into().unwrap()
}

#[test]
fn empty_name() {
    let server =
        TestServer::start(|_req| Response::new(200).with_set_cookie(Cookie::new("", value1())))
            .unwrap();
    assert_starts_with(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 500 ",
    );
}

#[test]
fn empty() {
    let server = TestServer::start(|_req| {
        Response::new(200).with_set_cookie(Cookie::new("name1", AsciiString::new()))
    })
    .unwrap();
    assert_starts_with(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nset-cookie: name1=; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\n\r\n",
    );
}

#[test]
fn one() {
    let server = TestServer::start(|_req| {
        Response::new(200).with_set_cookie(Cookie::new("name1", value1()))
    })
    .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nset-cookie: name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\n\r\n",
    );
}

#[test]
fn two() {
    let server = TestServer::start(|_req| {
        Response::new(200)
            .with_set_cookie(Cookie::new("name1", value1()))
            .with_set_cookie(Cookie::new("name2", value2()))
    })
    .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nset-cookie: name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\nset-cookie: name2=value2; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\n\r\n",
    );
}

#[test]
fn duplicate() {
    let server = TestServer::start(|_req| {
        Response::new(200)
            .with_set_cookie(Cookie::new("name1", value1()))
            .with_set_cookie(Cookie::new("name1", value2()))
    })
    .unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nset-cookie: name1=value1; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\nset-cookie: name1=value2; HttpOnly; Max-Age=2592000; SameSite=Strict; Secure\r\n\r\n",
    );
}
