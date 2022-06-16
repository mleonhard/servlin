#![cfg(feature = "urlencoded")]
mod test_util;

use crate::test_util::TestServer;
use serde::Deserialize;
use servlin::Response;

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
