extern crate core;

mod test_util;

use crate::test_util::TestServer;
use fixed_buffer::FixedBuf;
use futures_lite::AsyncWriteExt;
use safina_sync::Receiver;
use safina_timer::sleep_for;
use servlin::internal::{read_http_head, Head, HeadError, HttpError};
use servlin::{AsciiString, Response};
use std::time::Duration;
use test_util::{async_test, connected_streams};

#[test]
fn request_line() {
    let server = TestServer::start(|_req| Response::new(200)).unwrap();
    assert_eq!(server.exchange("").unwrap().as_str(), "",);
    assert_eq!(
        server.exchange("M / HTTP/1.1\r\n\r\n").unwrap().as_str(),
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n",
    );
    assert_eq!(
        server.exchange(" / HTTP/1.1\r\n\r\n").unwrap().as_str(),
        "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 31\r\n\r\nHttpError::MalformedRequestLine",
    );
}

#[test]
fn try_read_request_line() {
    Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\n\r\n")).unwrap();
    assert_eq!(
        Err(HeadError::Truncated),
        Head::try_read(&mut <FixedBuf<10>>::new())
    );
    for (expected_err, req) in [
        (HeadError::Truncated, ""),
        (HeadError::MalformedRequestLine, " / HTTP/1.1\r\n\r\n"),
        (HeadError::MalformedRequestLine, "M  HTTP/1.1\r\n\r\n"),
        (HeadError::MalformedRequestLine, "M / \r\n\r\n"),
        (HeadError::Truncated, "M / HTTP/1.1\n\r\n"),
        (HeadError::Truncated, "M / HTTP/1.1\r\n\r"),
        (
            HeadError::MalformedHeader,
            "M / HTTP/1.1\r\nM / HTTP/1.1\r\n\r\n",
        ),
    ] {
        let mut buf: FixedBuf<200> = FixedBuf::new();
        buf.write_bytes(req).unwrap();
        assert_eq!(Err(expected_err), Head::try_read(&mut buf), "{:?}", req,);
    }
}

#[test]
fn try_read_method() {
    assert_eq!(
        "M",
        Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\n\r\n",))
            .unwrap()
            .method
    );
    // TODO: Check all valid method chars.
}

#[test]
fn try_read_url() {
    assert_eq!(
        "/",
        Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\n\r\n",))
            .unwrap()
            .url
            .path()
    );
    assert_eq!(
        Err(HeadError::MalformedPath),
        Head::try_read(&mut FixedBuf::from(*b"M a HTTP/1.1\r\n\r\n",))
    );
    assert_eq!(
        Err(HeadError::MalformedRequestLine),
        Head::try_read(&mut FixedBuf::from(*b"M /\n HTTP/1.1\r\n\r\n",))
    );
    assert_eq!(
        Err(HeadError::MalformedRequestLine),
        Head::try_read(&mut FixedBuf::from(*b"M /  HTTP/1.1\r\n\r\n",))
    );
    assert_eq!(
        Err(HeadError::MalformedRequestLine),
        Head::try_read(&mut FixedBuf::from(*b"M / / HTTP/1.1\r\n\r\n",))
    );
}

#[test]
fn try_read_proto() {
    Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\n\r\n")).unwrap();
    for req in [
        "M / HTTP/1.0\r\n\r\n",
        "M / HTTP/1.2\r\n\r\n",
        "M / X\r\n\r\n",
    ] {
        let mut buf: FixedBuf<200> = FixedBuf::new();
        buf.write_bytes(req).unwrap();
        assert_eq!(
            Err(HeadError::UnsupportedProtocol),
            Head::try_read(&mut buf),
            "{:?}",
            req
        );
    }
}

#[test]
fn head_try_read_headers() {
    Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\na:b\r\n\r\n")).unwrap();
    for (expected, req) in [
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\n:v\r\n\r\n",
        ),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\nav\r\n\r\n",
        ),
        (Ok(vec!["".to_string()]), "M / HTTP/1.1\r\na:\r\n\r\n"),
        (Ok(vec!["b".to_string()]), "M / HTTP/1.1\r\na:b\r\n\r\n"),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\n a:b\r\n\r\n",
        ),
        // Strips value whitespace.
        (
            Ok(vec!["b".to_string()]),
            "M / HTTP/1.1\r\na: \t\rb\r\n\r\n",
        ),
        (
            Ok(vec!["b".to_string()]),
            "M / HTTP/1.1\r\na:b \t\r\r\n\r\n",
        ),
        // Keeps last duplicate.
        (
            Ok(vec!["1".to_string(), "2".to_string()]),
            "M / HTTP/1.1\r\na:1\r\nA:2\r\n\r\n",
        ),
        // Extra newlines
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\n\na:b\r\n\r\n",
        ),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\na:b\r\n\nc:d\r\n\r\n",
        ),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\na:b\r\n\n\r\n\r\n",
        ),
        // Extra carriage-returns
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\n\ra:b\r\n\r\n",
        ),
        (
            Ok(vec!["b".to_string()]),
            "M / HTTP/1.1\r\na:b\r\r\nc:d\r\n\r\n",
        ),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\na:b\r\n\rc:d\r\n\r\n",
        ),
        (
            Err(HeadError::MalformedHeader),
            "M / HTTP/1.1\r\na:b\r\n\r\r\n\r\n",
        ),
        (
            Ok(vec!["b".to_string()]),
            "M / HTTP/1.1\r\na:b\r\r\n\r\n\r\n",
        ),
    ] {
        let mut buf: FixedBuf<200> = FixedBuf::new();
        buf.write_bytes(req).unwrap();
        assert_eq!(
            expected,
            Head::try_read(&mut buf).map(|mut head| head
                .headers
                .remove_all("a")
                .into_iter()
                .map(AsciiString::into)
                .collect::<Vec<String>>()),
            "{:?}",
            req
        );
    }
    // Lookups are case-insensitive.
    assert_eq!(
        Some("CCdd2"),
        Head::try_read(&mut FixedBuf::from(*b"M / HTTP/1.1\r\nAAbb1:CCdd2\r\n\r\n",))
            .unwrap()
            .headers
            .get_only("aabb1")
            .map(AsciiString::as_str)
    );
    // Accepts all valid header name symbols.
    // https://datatracker.ietf.org/doc/html/rfc7230#section-3.2
    //     header-field   = field-name ":" OWS field-value OWS
    //     field-name     = token
    //     field-value    = *( field-content )
    //     field-content  = field-vchar [ 1*( SP / HTAB ) field-vchar ]
    //     field-vchar    = VCHAR
    //     OWS            = *( SP / HTAB )
    //     token          = 1*tchar
    //     tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    //                      / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    //                      / DIGIT / ALPHA
    //                      ; any VCHAR, except delimiters
    //     VCHAR is any visible ASCII character.
    assert_eq!(
        Some("! \"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"),
        Head::try_read(&mut FixedBuf::from(
            *(b"M / HTTP/1.1\r\n1#$%&'*+-.^_`|~0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ:! \"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\r\n\r\n")
        ))
            .unwrap()
            .headers
            .get_only("1#$%&'*+-.^_`|~0123456789abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz")
            .map(AsciiString::as_str)
    );
    // TODO: Add tests of non-ASCII chars in names and values.
}

#[test]
fn head_try_read_reads() {
    let mut buf: FixedBuf<100> = FixedBuf::new();
    buf.write_bytes("A /a HTTP/1.1\r\n\r\nB /b HTTP/1.1\r\n\r\n")
        .unwrap();
    let head = Head::try_read(&mut buf).unwrap();
    assert_eq!("A", head.method);
    assert_eq!("/a", head.url.path());
    let head = Head::try_read(&mut buf).unwrap();
    assert_eq!("B", head.method);
    assert_eq!("/b", head.url.path());
    assert_eq!(Err(HeadError::Truncated), Head::try_read(&mut buf));
}

async fn read_http_head_task() -> (async_net::TcpStream, Receiver<Result<Head, HttpError>>) {
    let (mut stream0, stream1) = connected_streams().await;
    let (sender, receiver) = safina_sync::sync_channel(5);
    safina_executor::spawn(async move {
        loop {
            let result = read_http_head(&mut <FixedBuf<1000>>::new(), &mut stream0).await;
            let result_is_err = result.is_err();
            if sender.send(result).is_err() || result_is_err {
                break;
            }
        }
    });
    (stream1, receiver)
}

#[test]
fn read_http_head_ok() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(b"M / HTTP/1.1\r\n\r\n").await.unwrap();
        let head = receiver.async_recv().await.unwrap().unwrap();
        assert_eq!("M", head.method);
        assert_eq!("/", head.url.path());
        assert!(head.headers.is_empty());
    });
}

#[test]
fn read_http_head_error() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(b"M / BADPROTO\r\n\r\n").await.unwrap();
        assert_eq!(
            Err(HttpError::UnsupportedProtocol),
            receiver.async_recv().await.unwrap()
        );
    });
}

#[test]
fn read_http_head_too_long() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(&[b'a'; 10_000]).await.unwrap();
        assert_eq!(
            Err(HttpError::HeadTooLong),
            receiver.async_recv().await.unwrap()
        );
    });
}

#[test]
fn read_http_head_truncated() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(b"M / HTTP/1.1\r\n\r").await.unwrap();
        drop(stream);
        assert_eq!(
            Err(HttpError::Truncated),
            receiver.async_recv().await.unwrap()
        );
    });
}

#[test]
fn read_http_head_multiple_writes() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(b"M / HTTP/1.1\r\n").await.unwrap();
        stream.flush().await.unwrap();
        sleep_for(Duration::from_millis(100)).await;
        stream.write_all(b"\r\n").await.unwrap();
        stream.flush().await.unwrap();
        let head = receiver.async_recv().await.unwrap().unwrap();
        assert_eq!("M", head.method);
        assert_eq!("/", head.url.path());
        assert!(head.headers.is_empty());
    });
}

#[test]
fn read_http_head_subsequent() {
    async_test(async {
        let (mut stream, mut receiver) = read_http_head_task().await;
        stream.write_all(b"M /1 HTTP/1.1\r\n\r\n").await.unwrap();
        assert_eq!(
            "/1",
            receiver.async_recv().await.unwrap().unwrap().url.path()
        );
        stream.write_all(b"M /2 HTTP/1.1\r\n\r\n").await.unwrap();
        assert_eq!(
            "/2",
            receiver.async_recv().await.unwrap().unwrap().url.path()
        );
        drop(stream);
        assert_eq!(
            Err(HttpError::Disconnected),
            receiver.async_recv().await.unwrap()
        );
        receiver.async_recv().await.unwrap_err();
    });
}

#[test]
fn head_derive() {
    let head1 = Head::try_read(&mut FixedBuf::from(
        *b"A /1 HTTP/1.1\r\nH1: V1\r\nh2:v2\r\n\r\n",
    ))
    .unwrap();
    let head2 = Head::try_read(&mut FixedBuf::from(*b"B /2 HTTP/1.1\r\n\r\n")).unwrap();
    // Clone
    let head1_clone = head1.clone();
    // Eq, PartialEq
    assert_eq!(head1, head1_clone);
    assert_ne!(head1, head2);
    // Debug
    assert_eq!(
        "Head{method=\"A\", path=\"/1\", headers={H1: \"V1\", h2: \"v2\"}}",
        format!("{:?}", head1).as_str()
    );
}
