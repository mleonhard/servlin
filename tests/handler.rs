#![cfg(feature = "internals")]

use crate::test_util::{assert_starts_with, TestServer};
use beatrice::Response;

mod test_util;

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
