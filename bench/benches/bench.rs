//! Run: cargo +nightly bench
use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use permit::Permit;
use safina::executor::Executor;
use servlin::{HttpServerBuilder, Response, socket_addr_127_0_0_1_any_port};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

const WARMUP: Duration = Duration::from_millis(50);
const MEASUREMENT: Duration = Duration::from_millis(200);

fn drop_connection_servlin(c: &mut Criterion) {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |_req| Response::drop_connection();
    let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(1000)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    let mut group = c.benchmark_group("group");
    group
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(WARMUP)
        .measurement_time(MEASUREMENT);
    group.bench_function("drop_connection_servlin", move |b| {
        b.iter(move || {
            let mut tcp_stream =
                std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
            tcp_stream.write_all(b"M / HTTP/1.1\r\n\r\n").unwrap();
            tcp_stream.shutdown(Shutdown::Write).unwrap();
            assert_eq!(0, tcp_stream.read(&mut [0u8; 1]).unwrap());
        });
    });
    group.finish();
}

fn get_body(addr: SocketAddr) {
    let mut tcp_stream =
        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
    tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    tcp_stream.shutdown(Shutdown::Write).unwrap();
    let mut body = String::new();
    tcp_stream.read_to_string(&mut body).unwrap();
    assert_eq!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 5\r\n\r\nbody1",
        &body
    );
    let _ = tcp_stream.shutdown(Shutdown::Both);
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

fn small_servlin(c: &mut Criterion) {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |_req| Response::text(200, "body1");
    let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(1000)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    let mut group = c.benchmark_group("group");
    group
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(WARMUP)
        .measurement_time(MEASUREMENT);
    group.bench_function("small_servlin", move |b| b.iter(move || get_body(addr)));
    group.finish();
}

fn client_timeout_servlin(c: &mut Criterion) {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |_req| {
        std::thread::sleep(Duration::from_secs(1));
        Response::text(200, "body1")
    };
    let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(1000)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    let mut group = c.benchmark_group("group");
    group
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(WARMUP)
        .measurement_time(MEASUREMENT);
    group.bench_function("client_timeout_servlin", move |b| {
        b.iter(move || {
            let mut tcp_stream =
                std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(5000)).unwrap();
            tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
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

criterion_group!(
    benches,
    client_timeout_servlin,
    drop_connection_servlin,
    small_servlin,
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
