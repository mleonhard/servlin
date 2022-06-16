#![cfg(feature = "json")]
mod test_util;

use crate::test_util::TestServer;
use serde_json::json;
use servlin::Response;

#[test]
fn json() {
    let server =
        TestServer::start(|_req| Response::json(200, json!({"key":123})).unwrap()).unwrap();
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=UTF-8\r\ncontent-length: 11\r\n\r\n{\"key\":123}",
    );
}
