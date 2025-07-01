mod test_util;

use crate::test_util::connected_streams;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use permit::Permit;
use safina::async_test;
use servlin::internal::{Token, handle_http_conn};
use servlin::{HttpConn, Request, Response};
use std::future::Future;
use std::io::ErrorKind;
use std::net::Shutdown;
use std::time::{Duration, Instant};
use temp_dir::TempDir;

const MILLIS_100: Duration = Duration::from_millis(100);

async fn handle_http_conn_task<F, Fut>(request_handler: F) -> async_net::TcpStream
where
    Fut: Future<Output = Response> + Send,
    F: FnOnce(Request) -> Fut + 'static + Send + Sync + Clone,
{
    let (stream0, stream1) = connected_streams().await;
    let addr = stream1.local_addr().unwrap();
    let temp_dir = TempDir::new().unwrap();
    safina::executor::spawn(async move {
        handle_http_conn(
            Permit::new(),
            Token::new(),
            HttpConn::new(addr, stream0),
            Some(temp_dir.path().to_path_buf()),
            64 * 1024,
            request_handler,
        )
        .await;
    });
    stream1
}

async fn read_response(
    stream: &mut async_net::TcpStream,
    timeout: Duration,
) -> Result<String, std::io::Error> {
    let deadline = Instant::now() + timeout;
    let mut buf = Vec::new();
    while Instant::now() < deadline {
        let mut chunk = [0_u8; 1024];
        let result = safina::timer::with_timeout(
            async { stream.read(&mut chunk).await },
            Duration::from_millis(10),
        )
        .await;
        match result {
            Err(safina::timer::DeadlineExceededError) => {}
            Ok(Ok(0)) => break,
            Ok(Ok(num_read)) => buf.extend(&chunk[..num_read]),
            Ok(Err(e)) => return Err(e),
        }
    }
    String::from_utf8(buf)
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "response is not UTF-8"))
}

#[async_test]
async fn handle_http_conn_ok() {
    let mut stream =
        handle_http_conn_task(|_req: Request| async { Response::text(200, "ok") }).await;
    stream.write_all(b"M / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 2\r\n\r\nok",
        read_response(&mut stream, MILLIS_100)
            .await
            .unwrap()
            .as_str()
    );
}

#[async_test]
async fn handle_http_conn_shutdown() {
    let mut stream =
        handle_http_conn_task(|_req: Request| async { Response::text(200, "ok") }).await;
    stream.shutdown(Shutdown::Write).unwrap();
    assert_eq!(
        "",
        read_response(&mut stream, MILLIS_100)
            .await
            .unwrap()
            .as_str()
    );
}

#[async_test]
async fn handle_http_conn_upload() {
    let mut stream = handle_http_conn_task(|req: Request| async move {
        let mut body_string = String::new();
        req.body
            .async_reader()
            .await
            .unwrap()
            .read_to_string(&mut body_string)
            .await
            .unwrap();
        Response::text(200, format!("read {body_string:?}"))
    })
    .await;
    stream
        .write_all(b"M / HTTP/1.1\r\ncontent-length:3\r\n\r\nabc")
        .await
        .unwrap();
    assert_eq!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 10\r\n\r\nread \"abc\"",
        read_response(&mut stream, MILLIS_100)
            .await
            .unwrap()
            .as_str()
    );
}

#[async_test]
async fn handle_http_conn_upload_large() {
    let mut stream = handle_http_conn_task(|req: Request| async move {
        if req.body.is_pending() {
            return Response::get_body_and_reprocess(10_000_000);
        }
        let mut body_string = String::new();
        req.body
            .async_reader()
            .await
            .unwrap()
            .read_to_string(&mut body_string)
            .await
            .unwrap();
        Response::text(200, format!("got {}", body_string.len()))
    })
    .await;
    stream
        .write_all(b"M / HTTP/1.1\r\ncontent-length:10000000\r\n\r\n")
        .await
        .unwrap();
    for _ in 0..10_000 {
        stream.write_all(&[b'a'; 1000]).await.unwrap();
    }
    stream.flush().await.unwrap();
    assert_eq!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 12\r\n\r\ngot 10000000",
        read_response(&mut stream, Duration::from_secs(3))
            .await
            .unwrap()
            .as_str()
    );
}
