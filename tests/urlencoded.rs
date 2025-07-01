#![cfg(feature = "urlencoded")]
mod test_util;

use crate::test_util::TestServer;
use serde::Deserialize;
use servlin::Response;

#[test]
fn parse_url() {
    let server = TestServer::start(|req| {
        #[derive(Deserialize)]
        struct Input {
            num: usize,
            msg: String,
        }
        let input: Input = match req.parse_url() {
            Ok(input) => input,
            Err(response) => return response,
        };
        assert_eq!(111, input.num);
        assert_eq!("aaa", input.msg.as_str());
        Response::redirect_303("/")
    })
    .unwrap();
    assert_eq!(
        server
            .exchange("M /?num=111&msg=aaa HTTP/1.1\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 303 See Other\r\ncontent-length: 0\r\nlocation: /\r\n\r\n",
    );
    assert_eq!(
        server
            .exchange("M /?num=not_an_integer&msg=aaa HTTP/1.1\r\n\r\n")
            .unwrap(),
        "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 51\r\n\r\nerror processing url: invalid digit found in string",
    );
    assert_eq!(
        server.exchange("M /? HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 41\r\n\r\nerror processing url: missing field `num`",
    );
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 41\r\n\r\nerror processing url: missing field `num`",
    );
}

#[test]
fn urlencoded() {
    let server = TestServer::start(|req| {
        #[derive(Deserialize)]
        struct Input {
            num: usize,
            msg: String,
        }
        let input: Input = match req.urlencoded() {
            Ok(input) => input,
            Err(response) => return response,
        };
        assert_eq!(123, input.num);
        assert_eq!("abc", input.msg.as_str());
        Response::redirect_303("/")
    })
    .unwrap();
    assert_eq!(
        server.exchange(
            "M / HTTP/1.1\r\ncontent-type: application/x-www-form-urlencoded; charset=UTF-8\r\ncontent-length: 15\r\n\r\nnum=123&msg=abc"
        ).unwrap(),
        "HTTP/1.1 303 See Other\r\ncontent-length: 0\r\nlocation: /\r\n\r\n",
    );
}
