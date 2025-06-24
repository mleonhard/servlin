//! Run: cargo +nightly bench
use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use fixed_buffer::{FixedBuf, MalformedInputError};
use permit::Permit;
use safe_regex::{Matcher1, regex};
use safina::executor::Executor;
use servlin::internal::escape_and_elide;
use servlin::{HttpServerBuilder, Request, Response, socket_addr_127_0_0_1_any_port};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const WARMUP: Duration = Duration::from_millis(50);
const MEASUREMENT: Duration = Duration::from_millis(10_000);

fn deframe_http_response(
    b: &[u8],
) -> Result<(usize, Option<core::ops::Range<usize>>), MalformedInputError> {
    let Some((idx, _)) = b.windows(4).enumerate().find(|(_n, w)| w == b"\r\n\r\n") else {
        return Ok((0, None));
    };
    let head_len = idx + 4;
    let head = &b[0..head_len];
    let lowercase_head = String::from_utf8_lossy(head).to_ascii_lowercase();
    let matcher: Matcher1<_> = regex!(br".*\r\ncontent-length: ([0-9]+)\r\n.*");
    let Some((content_length_str_bytes,)) = matcher.match_slices(lowercase_head.as_bytes()) else {
        return Err(MalformedInputError(format!(
            "response is missing content-length header: {}",
            escape_and_elide(head, 100),
        )));
    };
    let content_length_str = str::from_utf8(content_length_str_bytes).unwrap();
    let content_length = usize::from_str(content_length_str).unwrap();
    let response_length = head_len + content_length;
    if b.len() < response_length {
        return Ok((0, None));
    }
    Ok((response_length, Some(0..response_length)))
}

fn start_load_threads(permit: Permit, addr: SocketAddr, num_threads: usize) {
    for _ in 0..num_threads {
        let permit_clone = permit.clone();
        std::thread::Builder::new()
            .spawn(move || {
                let mut buf: FixedBuf<65536> = FixedBuf::default();
                loop {
                    if permit_clone.is_revoked() {
                        return;
                    }
                    let mut tcp_stream = match std::net::TcpStream::connect_timeout(
                        &addr,
                        Duration::from_millis(1000),
                    ) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("error connecting: {}", e);
                            std::thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                    };
                    buf.clear();
                    for _ in 0..500 {
                        if permit_clone.is_revoked() {
                            return;
                        }
                        if let Err(e) =
                            tcp_stream.write_all(b"GET /small_response HTTP/1.1\r\n\r\n")
                        {
                            eprintln!("error writing: {}", e);
                            break;
                        }
                        match buf.read_frame(&mut tcp_stream, deframe_http_response) {
                            Ok(None) => {
                                eprintln!("error reading: connection closed");
                                break;
                            }
                            Ok(Some(r)) => r,
                            Err(e) => {
                                eprintln!("error reading: {}", e);
                                break;
                            }
                        };
                    }
                    let _ = tcp_stream.shutdown(Shutdown::Both);
                }
            })
            .unwrap();
    }
}

fn do_http_request(addr: SocketAddr, path: &'static str, expected_response: &'static str) {
    let mut tcp_stream =
        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
    tcp_stream
        .set_read_timeout(Some(Duration::from_millis(5000)))
        .unwrap();
    tcp_stream
        .write_all(format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes())
        .unwrap();
    tcp_stream.shutdown(Shutdown::Write).unwrap();
    let mut body = String::new();
    tcp_stream.read_to_string(&mut body).unwrap();
    assert_eq!(&body, expected_response);
    let _ = tcp_stream.shutdown(Shutdown::Both);
}

fn measure_servlin(c: &mut Criterion) {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |req: Request| match req.url.path() {
        "/drop_connection" => Response::drop_connection(),
        "/small_response" => Response::text(200, "small_response1"),
        "/slow" => {
            std::thread::sleep(Duration::from_secs(1));
            Response::ok_200()
        }
        _ => Response::not_found_404(),
    };
    let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(10000)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    start_load_threads(permit.new_sub(), addr, 2000);
    let mut group = c.benchmark_group("measure_servlin");
    group
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(WARMUP)
        .measurement_time(MEASUREMENT);
    group.bench_function("small_response", move |b| {
        b.iter(move || do_http_request(addr, "/small_response", "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 15\r\n\r\nsmall_response1"));
    });
    group.bench_function("drop_connection", move |b| {
        b.iter(move || do_http_request(addr, "/drop_connection", ""));
    });
    group.bench_function("timeout", move |b| {
        b.iter(move || {
            let mut tcp_stream =
                std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
            tcp_stream.write_all(b"GET /slow HTTP/1.1\r\n\r\n").unwrap();
            tcp_stream
                .set_read_timeout(Some(Duration::from_millis(10)))
                .unwrap();
            let kind = tcp_stream
                .read_to_string(&mut String::new())
                .unwrap_err()
                .kind();
            assert!(
                kind == ErrorKind::TimedOut || kind == ErrorKind::WouldBlock,
                "kind={kind:?}"
            );
        });
    });
    group.finish();
}

// Crashes with:
// thread 'tokio-runtime-worker' panicked at /Users/user/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.45.1/src/task/local.rs:418:29:
// `spawn_local` called from outside of a `task::LocalSet` or LocalRuntime
// stack backtrace:
//    0: __rustc::rust_begin_unwind
//    1: core::panicking::panic_fmt
//
//    2: tokio::task::local::spawn_local
//    3: ntex_server::manager::ServerManager<F>::start
//    4: ntex_server::net::builder::ServerBuilder::run
//    5: bench::small_tokio::{{closure}}
//
// fn small_tokio(c: &mut Criterion) {
//     use std::net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4};
//     use ntex::web;
//     use ntex::web::{App, HttpRequest};
//     use tokio::runtime::Builder;
//     async fn handler(_req: HttpRequest) -> &'static str {
//         "body1"
//     }
//     let rt = Builder::new_multi_thread().enable_time().build().unwrap();
//     let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3010));
//     let _server = rt.spawn(async {
//         tokio::time::sleep(Duration::from_millis(1)).await;
//         web::server(|| App::new().service(web::resource("/").to(handler)))
//             .bind("127.0.0.1:3010")
//             .unwrap()
//             .run()
//             .await
//     });
//     let mut group = c.benchmark_group("group");
//     group
//         .sampling_mode(SamplingMode::Flat)
//         .warm_up_time(WARMUP)
//         .measurement_time(MEASUREMENT);
//     group.bench_function("small_tokio", move |b| b.iter(move || get_body(addr)));
//     group.finish();
// }

criterion_group!(
    benches,
    measure_servlin,
    // small_tokio,
);
criterion_main!(benches);

// #![feature(test)]
// extern crate test;
// use test::Bencher;
//
// #[bench]
// fn connect(b: &mut Bencher) {
//     let permit = Permit::new();
//     safina::timer::start_timer_thread();
//     let handler = |_req| Response::drop_connection();
//     let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
//     let (addr, _stopped_receiver) = executor
//         .block_on(
//             HttpServerBuilder::new()
//                 .listen_addr(socket_addr_127_0_0_1_any_port())
//                 .max_conns(1000)
//                 .small_body_len(64 * 1024)
//                 .permit(permit.new_sub())
//                 .spawn(handler),
//         )
//         .unwrap();
//     b.iter(move || {
//         let mut tcp_stream =
//             std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
//         tcp_stream.write_all(b"M / HTTP/1.1\r\n\r\n").unwrap();
//         assert_eq!(0, tcp_stream.read(&mut [0u8; 1]).unwrap());
//     });
// }
