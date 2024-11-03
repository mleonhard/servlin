mod test_util;

use fixed_buffer::FixedBuf;
use futures_lite::AsyncWriteExt;
use safina::async_test;
use safina::sync::Receiver;
use servlin::internal::{read_http_request, HttpError};
use servlin::{AsciiString, ContentType, Request, RequestBody};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use test_util::connected_streams;

fn addr1() -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1))
}

async fn call_read(b: impl AsRef<[u8]>) -> Result<Request, HttpError> {
    let mut buf: FixedBuf<1000> = FixedBuf::new();
    std::io::Write::write_all(&mut buf, b.as_ref()).unwrap();
    read_http_request(addr1(), &mut buf, <FixedBuf<0>>::new()).await
}

#[async_test]
async fn head() {
    let req = call_read("M /1 HTTP/1.1\r\nHeader1: Val1\r\n\r\n")
        .await
        .unwrap();
    assert_eq!(addr1(), req.remote_addr);
    assert_eq!("M", req.method());
    assert_eq!("/1", req.url.path());
    assert_eq!(
        Some("Val1"),
        req.headers.get_only("header1").map(AsciiString::as_str)
    );
}

#[async_test]
async fn content_type() {
    assert_eq!(
        ContentType::None,
        call_read("M / HTTP/1.1\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        ContentType::PlainText,
        call_read("M / HTTP/1.1\r\nCONTENT-type: text/plain\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        ContentType::PlainText,
        call_read("M / HTTP/1.1\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        ContentType::PlainText,
        call_read("M / HTTP/1.1\r\nContent-Type: text/plain; charset=set1\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        // Case-sensitive.
        ContentType::String("Text/plain".to_string()),
        call_read("M / HTTP/1.1\r\nCONTENT-type: Text/plain\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        // Unknown type
        ContentType::String("Type1".to_string()),
        call_read("M / HTTP/1.1\r\nContent-Type: Type1\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
    assert_eq!(
        // Unknown type with parameter.
        ContentType::String("type1; param1=val1".to_string()),
        call_read("M / HTTP/1.1\r\nContent-Type: type1; param1=val1\r\n\r\n")
            .await
            .unwrap()
            .content_type
    );
}

#[async_test]
async fn expect_continue() {
    let req = call_read("M / HTTP/1.1\r\n\r\n").await.unwrap();
    assert!(!req.expect_continue);
    assert_eq!(&RequestBody::empty(), &req.body);

    let req = call_read("M / HTTP/1.1\r\nExpect: 100-continue\r\n\r\nabc")
        .await
        .unwrap();
    assert!(req.expect_continue);
    assert_eq!(&RequestBody::PendingUnknown, &req.body);

    let req = call_read("M / HTTP/1.1\r\nexpect: 100-continue\r\ncontent-length: 3\r\n\r\nabc")
        .await
        .unwrap();
    assert!(req.expect_continue);
    assert_eq!(&RequestBody::PendingKnown(3), &req.body);
}

#[async_test]
async fn transfer_encoding() {
    let req = call_read("M / HTTP/1.1\r\n\r\n").await.unwrap();
    assert!(!req.chunked);
    assert!(!req.gzip);
    assert_eq!(&RequestBody::empty(), &req.body);

    let req = call_read("POST / HTTP/1.1\r\n\r\n").await.unwrap();
    assert!(!req.chunked);
    assert!(!req.gzip);
    assert_eq!(&RequestBody::PendingUnknown, &req.body);

    let req = call_read("M / HTTP/1.1\r\ntransfer-encoding: chunked\r\n\r\n")
        .await
        .unwrap();
    assert!(req.chunked);
    assert!(!req.gzip);
    assert_eq!(&RequestBody::PendingUnknown, &req.body);

    let req = call_read("M / HTTP/1.1\r\ntransfer-encoding: gzip\r\n\r\n")
        .await
        .unwrap();
    assert!(!req.chunked);
    assert!(req.gzip);
    assert_eq!(&RequestBody::PendingUnknown, &req.body);

    let req = call_read("M / HTTP/1.1\r\ntransfer-encoding: gzip, chunked\r\n\r\n")
        .await
        .unwrap();
    assert!(req.chunked);
    assert!(req.gzip);
    assert_eq!(&RequestBody::PendingUnknown, &req.body);

    let req = call_read("M / HTTP/1.1\r\ntransfer-encoding: gzip\r\ncontent-length:10\r\n\r\n")
        .await
        .unwrap();
    assert!(!req.chunked);
    assert!(req.gzip);
    assert_eq!(&RequestBody::PendingKnown(10), &req.body);
}

#[async_test]
async fn content_length() {
    let req = call_read("M / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!(None, req.content_length);
    assert_eq!(&RequestBody::empty(), &req.body);

    let req = call_read("M / HTTP/1.1\r\ncontent-length: 0\r\n\r\n")
        .await
        .unwrap();
    assert_eq!(Some(0), req.content_length);
    assert_eq!(&RequestBody::empty(), &req.body);

    let req = call_read("M / HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc")
        .await
        .unwrap();
    assert_eq!(Some(3), req.content_length);
    assert_eq!(&RequestBody::PendingKnown(3), &req.body);

    assert_eq!(
        Err(HttpError::InvalidContentLength),
        call_read("M / HTTP/1.1\r\ncontent-length: a\r\n\r\n").await
    );
    assert_eq!(
        Err(HttpError::InvalidContentLength),
        call_read("M / HTTP/1.1\r\ncontent-length: -1\r\n\r\n").await
    );

    let req = call_read("M / HTTP/1.1\r\ncontent-length: 18446744073709551615\r\n\r\n")
        .await
        .unwrap();
    assert_eq!(Some(u64::MAX), req.content_length);
    assert_eq!(&RequestBody::PendingKnown(u64::MAX), &req.body);

    assert_eq!(
        Err(HttpError::InvalidContentLength),
        call_read("M / HTTP/1.1\r\ncontent-length: 18446744073709551616\r\n\r\n").await
    );
}

#[async_test]
async fn method() {
    let req = call_read("M / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!(&RequestBody::empty(), &req.body);

    let req = call_read("POST / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!(&RequestBody::PendingUnknown, &req.body);
}

async fn read_http_request_task() -> (async_net::TcpStream, Receiver<Result<Request, HttpError>>) {
    let (mut stream0, stream1) = connected_streams().await;
    let addr = stream1.local_addr().unwrap();
    let (sender, receiver) = safina::sync::sync_channel(10);
    safina::executor::spawn(async move {
        let mut buf = <FixedBuf<1000>>::new();
        loop {
            match read_http_request(addr, &mut buf, &mut stream0).await {
                Err(HttpError::Disconnected) => break,
                result => {
                    let _ignored = sender.send(result);
                }
            }
        }
    });
    (stream1, receiver)
}

#[async_test]
async fn read_http_request_ok() {
    let (mut stream, mut receiver) = read_http_request_task().await;
    stream.write_all(b"M / HTTP/1.1\r\n\r\n").await.unwrap();
    let req = receiver.async_recv().await.unwrap().unwrap();
    assert_eq!("M", req.method());
    assert_eq!("/", req.url().path());
    drop(stream);
    receiver.async_recv().await.unwrap_err();
}

#[async_test]
async fn read_http_request_multiple_writes() {
    let (mut stream, mut receiver) = read_http_request_task().await;
    stream.write_all(b"A / HTTP/1.1\r\n\r\n").await.unwrap();
    stream.flush().await.unwrap();
    safina::timer::sleep_for(Duration::from_millis(100)).await;
    stream.write_all(b"B / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!("A", receiver.async_recv().await.unwrap().unwrap().method());
    assert_eq!("B", receiver.async_recv().await.unwrap().unwrap().method());
    stream.write_all(b"C / HTTP/1.1\r\n\r\n").await.unwrap();
    drop(stream);
    assert_eq!("C", receiver.async_recv().await.unwrap().unwrap().method());
}
