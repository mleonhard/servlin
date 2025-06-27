//! Run: cargo run --release
//! 2 threads: small_rps = 80200, medium_rps = 105400, large_rps = 10600
//! 4 threads: small_rps = 77800, medium_rps = 109000, large_rps = 9550

use crossbeam_channel::{Receiver, Sender};
use fixed_buffer::{FixedBuf, MalformedInputError};
use permit::Permit;
use safe_regex::{Matcher1, regex};
use safina::executor::Executor;
use servlin::internal::escape_and_elide;
use servlin::{HttpServerBuilder, Request, Response, socket_addr_127_0_0_1_any_port};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::thread::Thread;
use std::time::{Duration, Instant};

fn connect(addr: SocketAddr) -> Result<TcpStream, String> {
    let tcp_stream = TcpStream::connect_timeout(&addr, Duration::from_millis(5000))
        .map_err(|e| format!("connect error: {e}"))?;
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(10000)));
    let _ = tcp_stream.set_write_timeout(Some(Duration::from_millis(10000)));
    Ok(tcp_stream)
}

pub struct ThreadWaker(pub Thread);
impl Drop for ThreadWaker {
    fn drop(&mut self) {
        self.0.unpark();
    }
}

struct Ctx {
    waiting_thread_count: AtomicUsize,
    addr: SocketAddr,
    f: Box<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)>,
    slip_nanos: AtomicU64,
    stage_rx: Receiver<(Receiver<Instant>, Sender<Result<Duration, String>>)>,
}

fn measure_once(ctx: &Arc<Ctx>, opt_conn: Option<TcpStream>) -> Result<Option<TcpStream>, String> {
    let mut conn = match opt_conn {
        None => connect(ctx.addr).map_err(|e| format!("error connecting: {e}"))?,
        Some(conn) => conn,
    };
    (&ctx.f)(&mut conn)?;
    Ok(Some(conn))
}

// TODO: Measure connect time separately.

fn measure_stage(
    ctx: &Arc<Ctx>,
    token_rx: Receiver<Instant>,
    result_tx: Sender<Result<Duration, String>>,
    mut opt_conn: Option<TcpStream>,
) -> Option<TcpStream> {
    loop {
        ctx.waiting_thread_count.fetch_add(1, AcqRel);
        let start_time = match token_rx.recv() {
            Ok(t) => t,
            Err(..) => return opt_conn, // Empty and disconnected.
        };
        ctx.waiting_thread_count.fetch_sub(1, AcqRel);
        let now = Instant::now();
        let slippage = now.saturating_duration_since(start_time);
        if !slippage.is_zero() {
            let slippage_nanos = u64::try_from(slippage.as_nanos()).unwrap();
            ctx.slip_nanos.fetch_add(slippage_nanos, AcqRel);
        }
        let wait_time = start_time.saturating_duration_since(now);
        if !wait_time.is_zero() {
            std::thread::sleep(wait_time);
        }
        let before = Instant::now();
        match measure_once(ctx, opt_conn.take()) {
            Ok(Some(conn)) => opt_conn = Some(conn),
            Ok(None) => {}
            Err(e) => {
                let _ = result_tx.send(Err(e));
                continue;
            }
        }
        let elapsed = before.elapsed();
        let _ = result_tx.send(Ok(elapsed));
    }
}

fn measure_thread(ctx: &Arc<Ctx>) {
    let mut opt_conn = None;
    for (token_rx, result_tx) in ctx.stage_rx.iter() {
        opt_conn = measure_stage(ctx, token_rx, result_tx, opt_conn.take());
    }
}

pub fn measure_tcp_rps(
    num_threads: usize,
    addr: SocketAddr,
    error_limit: f32,
    time_limits: Vec<(f32, Duration)>,
    f: impl 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>),
) -> usize {
    let (stage_tx, stage_rx) = crossbeam_channel::bounded(num_threads);
    let ctx = Arc::new(Ctx {
        waiting_thread_count: AtomicUsize::new(0),
        addr,
        f: Box::new(f),
        slip_nanos: AtomicU64::new(0),
        stage_rx,
    });
    for _ in 0..num_threads {
        let ctx_clone = Arc::clone(&ctx);
        std::thread::Builder::new()
            .spawn(move || measure_thread(&ctx_clone))
            .unwrap();
    }
    let mut rps_range = 0..usize::MAX;
    loop {
        // Begin stage.
        let mut token_txs = Vec::new();
        let mut token_rxs = Vec::new();
        let mut result_txs = Vec::new();
        let mut result_rxs = Vec::new();
        let num_channels = 10.min(num_threads);
        let buffer_size = 10_000_000 / num_channels;
        for _ in 0..num_channels {
            let (token_tx, token_rx) = crossbeam_channel::bounded::<Instant>(buffer_size);
            token_txs.push(token_tx);
            token_rxs.push(token_rx);
            let (result_tx, result_rx) =
                crossbeam_channel::bounded::<Result<Duration, String>>(buffer_size);
            result_txs.push(result_tx);
            result_rxs.push(result_rx);
        }
        for n in 0..num_threads {
            let channel_num = n % num_channels;
            let _ = stage_tx.send((
                token_rxs[channel_num].clone(),
                result_txs[channel_num].clone(),
            ));
        }
        drop(result_txs);
        // Queue work.
        let rps_target = match (rps_range.start, rps_range.end) {
            (0, usize::MAX) => 100,
            (0, n) => 1.max(n / 2),
            (n, usize::MAX) => n * 4,
            (a, b) => (a + b) / 2,
        };
        let nanos_per_request = 1_000_000_000 / rps_target;
        let start = Instant::now().add(Duration::from_millis(2000));
        let num_requests = 100.max(rps_target);
        println!("rps_range={rps_range:?} rps_target={rps_target} request_count={num_requests}");
        for n in 0..num_requests {
            let offset_nanos = n * nanos_per_request;
            let offset_nanos_u64 = u64::try_from(offset_nanos).unwrap();
            let offset = Duration::from_nanos(offset_nanos_u64);
            let channel_num = n % num_channels;
            let time = start.add(offset);
            let _ = token_txs[channel_num].send(time);
        }
        drop(token_txs);
        // Read results
        let mut durations = Vec::new();
        let mut error_count = 0usize;
        let mut error_counts = HashMap::new();
        for result_rx in result_rxs {
            for result in result_rx {
                match result {
                    Ok(duration) => durations.push(duration),
                    Err(e) => {
                        error_count += 1;
                        error_counts
                            .entry(e)
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                    }
                }
            }
        }
        let error_ratio = (error_count as f32) / (num_requests as f32);
        durations.sort();
        let mut satisfies_constraints = true;
        for (target_percentile, max_duration) in &time_limits {
            if let Some(idx) = durations
                .iter()
                .rposition(|duration| duration <= max_duration)
            {
                let percentile = (idx as f32) / (durations.len() as f32);
                println!(
                    "response time {max_duration:?} is P{:.4}",
                    100f32 * percentile
                );
                if percentile < *target_percentile {
                    satisfies_constraints = false;
                }
            } else {
                println!("response time {max_duration:?} is P0");
                satisfies_constraints = false;
            }
        }
        let slip_nanos = ctx.slip_nanos.swap(0, AcqRel);
        let mean_slip_nanos = slip_nanos / (num_requests as u64);
        let mean_slip = Duration::from_nanos(mean_slip_nanos);
        println!("error_ratio={error_ratio} error_counts={error_counts:?} mean_slip={mean_slip:?}");
        if Duration::from_millis(100) < mean_slip {
            satisfies_constraints = false;
        }
        if error_limit < error_ratio {
            satisfies_constraints = false;
        }
        if satisfies_constraints {
            rps_range.start = rps_target;
        } else {
            rps_range.end = rps_target;
        }
        if 100 < (rps_range.end / (rps_range.end - rps_range.start)) {
            break;
        }
    }
    dbg! {&rps_range};
    rps_range.end
}

pub fn measure(
    status: impl ToString,
    f: impl FnOnce() -> Result<(), String>,
) -> (String, Duration) {
    let before = Instant::now();
    let result = f();
    let elapsed = before.elapsed();
    (result.err().unwrap_or_else(|| status.to_string()), elapsed)
}

fn deframe_http_head(
    b: &[u8],
) -> Result<(usize, Option<core::ops::Range<usize>>), MalformedInputError> {
    let Some((idx, _)) = b.windows(4).enumerate().find(|(_n, w)| w == b"\r\n\r\n") else {
        return Ok((0, None));
    };
    Ok((idx + 4, Some(0..idx)))
}

fn get_content_length(head: &[u8]) -> Result<usize, String> {
    let lowercase_head = String::from_utf8_lossy(head).to_ascii_lowercase();
    let matcher: Matcher1<_> = regex!(br".*\r\ncontent-length: ([0-9]+)(?:\r\n.*)?");
    let Some((content_length_str_bytes,)) = matcher.match_slices(lowercase_head.as_bytes()) else {
        return Err(format!(
            "response is missing content-length header: {}",
            escape_and_elide(head, 100),
        ));
    };
    let content_length_str = str::from_utf8(content_length_str_bytes).unwrap();
    let content_length = usize::from_str(content_length_str).unwrap();
    Ok(content_length)
}

fn do_http_request(
    conn: &mut TcpStream,
    path: &'static str,
    expected_body: Option<&'static str>,
) -> Result<(), String> {
    conn.write_all(format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes())
        .map_err(|e| format!("error writing: {e}"))?;
    let mut buf: FixedBuf<4096> = FixedBuf::default();
    let head = match buf.read_frame(conn, deframe_http_head) {
        Ok(None) => return Err("error reading: connection closed".to_string()),
        Ok(Some(head)) => head,
        Err(e) => return Err(format!("error reading: {e}")),
    };
    if !head.starts_with(b"HTTP/1.1 200 OK\r\n") {
        return Err(format!(
            "unexpected response: {}",
            escape_and_elide(head, 100)
        ));
    }
    let mut content_length = get_content_length(&head)?;
    if let Some(expected_response) = expected_body {
        let deframe_expected_bytes = |b: &[u8]| {
            if content_length <= b.len() {
                Ok((content_length, Some(0..content_length)))
            } else {
                Ok((0, None))
            }
        };
        match buf.read_frame(conn, deframe_expected_bytes) {
            Ok(None) => return Err("error reading: connection closed".to_string()),
            Ok(Some(b)) if b == expected_response.as_bytes() => {}
            Ok(Some(b)) => {
                return Err(format!("unexpected response: {}", escape_and_elide(b, 100)));
            }
            Err(e) => return Err(format!("error reading: {e}")),
        };
    } else {
        content_length -= buf.len();
        buf.clear();
        let writable = buf.writable();
        while content_length > 0 {
            let chunk_size = writable.len().min(content_length);
            let target = &mut writable[..chunk_size];
            conn.read_exact(target)
                .map_err(|e| format!("error reading: {e}"))?;
            content_length -= chunk_size;
        }
    }
    Ok(())
}

const MEDIUM_RESPONSE: [u8; 16384] = [b'M'; 16384];
const LARGE_RESPONSE: [u8; 512 * 1024] = [b'L'; 512 * 1024];

fn main() {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |req: Request| match req.url.path() {
        "/drop_connection" => Response::drop_connection(),
        "/small" => Response::text(200, "small_response1"),
        "/medium" => Response::text(200, MEDIUM_RESPONSE),
        "/large" => Response::text(200, LARGE_RESPONSE),
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
    let small_rps = measure_tcp_rps(
        10000,
        addr,
        0.1,
        vec![(0.99, Duration::from_millis(100))],
        move |conn| do_http_request(conn, "/small", Some("small_response1")),
    );
    dbg! {small_rps};
    let medium_rps = measure_tcp_rps(
        2000,
        addr,
        0.1,
        vec![(0.99, Duration::from_millis(100))],
        move |conn| do_http_request(conn, "/medium", None),
    );
    dbg! {medium_rps};
    let large_rps = measure_tcp_rps(
        2000,
        addr,
        0.1,
        vec![(0.99, Duration::from_millis(200))],
        move |conn| do_http_request(conn, "/large", None),
    );
    dbg! {large_rps};
    println!("small_rps = {small_rps}, medium_rps = {medium_rps}, large_rps = {large_rps}");

    //                             move |conn: &mut TcpStream| {
    // let mut result = Vec::new();
    // result.push(measure("small", || {
    //     do_http_request(conn, "/small_response", Some("small_response1"))
    // }));
    // result.push(measure("medium", || do_http_request(conn, "/medium", None)));
    // result.push(measure("large", || do_http_request(conn, "/large", None)));
    // match rand_range(0..100) {
    //     0 => result.push(measure("drop_connection", || {
    //         conn.write_all(format!("GET /drop_connection HTTP/1.1\r\n\r\n").as_bytes())
    //             .map_err(|e| format!("error writing: {e}"))?;
    //         let mut body = Vec::new();
    //         conn.read_to_end(&mut body)
    //             .map_err(|e| format!("error reading: {e}"))?;
    //         if !body.is_empty() {
    //             return Err(format!(
    //                 "unexpected response: {}",
    //                 escape_and_elide(&body, 100)
    //             ));
    //         }
    //         Ok(())
    //     })),
    //     1 => result.push(measure("timeout", || {
    //         conn.write_all(format!("GET /slow HTTP/1.1\r\n\r\n").as_bytes())
    //             .map_err(|e| format!("error writing: {e}"))?;
    //         conn.set_read_timeout(Some(Duration::from_millis(100)))
    //             .unwrap();
    //         let mut body = Vec::new();
    //         let _ = conn.read_to_end(&mut body);
    //         if !body.is_empty() {
    //             return Err(format!(
    //                 "unexpected response: {}",
    //                 escape_and_elide(&body, 100)
    //             ));
    //         }
    //         Ok(())
    //     })),
    //     _ => {}
    // }
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

// criterion_group!(
//     benches,
//     measure_servlin,
//     // small_tokio,
// );
// criterion_main!(benches);

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
