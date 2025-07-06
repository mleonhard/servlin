//! ```
//! Run: cargo run --release
//! 2 threads: small_rps = 80200, medium_rps = 105400, large_rps = 10600
//! 4 threads: small_rps = 77800, medium_rps = 109000, large_rps = 9550
//! ```

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::items_after_statements)]
use fixed_buffer::{FixedBuf, MalformedInputError};
use permit::Permit;
use safe_regex::{Matcher1, regex};
use safina::executor::Executor;
use servlin::internal::escape_and_elide;
use servlin::{HttpServerBuilder, Request, Response, socket_addr_127_0_0_1_any_port};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

async fn measure_once(
    ctx: &Arc<Ctx>,
    opt_conn: Option<TcpStream>,
) -> Result<Option<TcpStream>, String> {
    let conn = match opt_conn {
        None => timeout(Duration::from_millis(1000), TcpStream::connect(ctx.addr))
            .await
            .map_err(|e| format!("connect error: {e}"))?
            .map_err(|e| format!("connect error: {e}"))?,
        Some(conn) => conn,
    };
    let conn = timeout(Duration::from_millis(1000), (ctx.f)(conn))
        .await
        .map_err(|_| "timeout error".to_string())??;
    Ok(Some(conn))
}

async fn measure_task(ctx: Arc<Ctx>) {
    let mut opt_conn = None;
    loop {
        let request_spacing = Duration::from_nanos(ctx.request_spacing_nanos.load(Acquire));
        let before = Instant::now();
        match measure_once(&ctx, opt_conn.take()).await {
            Ok(Some(conn)) => {
                ctx.add_success(before.elapsed());
                opt_conn = Some(conn);
            }
            Ok(None) => ctx.add_success(before.elapsed()),
            Err(e) => ctx.add_error(e),
        }
        ctx.duration_ns
            .fetch_add(before.elapsed().as_nanos() as u64, AcqRel);
        let next = before + request_spacing;
        sleep_until(next);
    }
}

fn sleep_until(t: Instant) {
    let wait_time = t.saturating_duration_since(Instant::now());
    if !wait_time.is_zero() {
        std::thread::sleep(wait_time);
    }
}

type AsyncMeasureFn = dyn 'static
    + Sync
    + Send
    + (Fn(TcpStream) -> Pin<Box<dyn Sync + Send + Future<Output = Result<TcpStream, String>>>>);
struct Ctx {
    addr: SocketAddr,
    count_slow: AtomicU64,
    count_all: AtomicU64,
    count_error: AtomicU64,
    f: Box<AsyncMeasureFn>,
    request_spacing_nanos: AtomicU64,
    time_limit: Duration,
    duration_ns: AtomicU64,
    //error_counts: Mutex<HashMap<String, Arc<AtomicUsize>>>,
}
impl Ctx {
    pub fn add_success(self: &Arc<Self>, duration: Duration) {
        self.count_all.fetch_add(1, AcqRel);
        if self.time_limit < duration {
            self.count_slow.fetch_add(1, AcqRel);
        }
    }

    pub fn add_error(self: &Arc<Self>, _e: String) {
        self.count_all.fetch_add(1, AcqRel);
        self.count_error.fetch_add(1, AcqRel);
    }
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn measure_tcp_rps(
    addr: SocketAddr,
    error_limit: f32,
    time_limit: (f32, Duration),
    f: Box<AsyncMeasureFn>,
) -> u64 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let ctx = Arc::new(Ctx {
        addr,
        count_slow: AtomicU64::default(),
        count_all: AtomicU64::default(),
        count_error: AtomicU64::default(),
        f: Box::new(f),
        request_spacing_nanos: AtomicU64::new(100_000_000),
        time_limit: time_limit.1,
        duration_ns: AtomicU64::default(),
    });
    let mut num_tasks = 1u64;
    for _ in 0..num_tasks {
        rt.spawn(measure_task(Arc::clone(&ctx)));
    }
    let mut now = Instant::now();
    let mut target_rps = 100u64;
    let mut recent_statuses = VecDeque::new();
    let mut recent_rps = VecDeque::new();
    loop {
        ctx.request_spacing_nanos
            .store((1_000_000_000 * num_tasks) / target_rps, Release);
        now += Duration::from_millis(100);
        sleep_until(now);
        ctx.count_slow.store(0, Release);
        ctx.count_error.store(0, Release);
        ctx.count_all.store(0, Release);
        now += Duration::from_millis(100);
        sleep_until(now);
        let count_slow = ctx.count_slow.swap(0, AcqRel);
        let count_error = ctx.count_error.swap(0, AcqRel);
        let count_all = ctx.count_all.swap(0, AcqRel);
        let mean_duration_ns = ctx.duration_ns.swap(0, AcqRel) / 1.max(count_all);
        let mut ok = true;
        let error_proportion = count_error as f32 / count_all.max(1) as f32;
        if error_limit < error_proportion {
            ok = false;
        }
        let fast_proportion = (count_all - count_slow) as f32 / count_all.max(1) as f32;
        if fast_proportion < time_limit.0 {
            ok = false;
        }
        let rps = count_all * 10;
        println!(
            "ok={ok:5} tasks={num_tasks} errors={error_proportion:.3} fast=P{:.3} target_rps={target_rps} rps={rps}",
            100.0 * fast_proportion,
        );
        if count_all == 0 {
            continue;
        }
        let total_duration_per_second = mean_duration_ns * target_rps;
        let needed_tasks = total_duration_per_second / 1_000_000_000;
        if ok && num_tasks < 1000 && num_tasks < needed_tasks {
            rt.spawn(measure_task(Arc::clone(&ctx)));
            num_tasks += 1;
        }
        // msg.push_str(&format!(" err={:?}", combine_error_counts(results)));
        // Adjust RPS.
        recent_statuses.push_front(ok);
        const NUM_STATUSES: usize = 15;
        while NUM_STATUSES < recent_statuses.len() {
            recent_statuses.pop_back();
        }
        recent_rps.push_front(target_rps);
        while NUM_STATUSES < recent_rps.len() {
            recent_rps.pop_back();
        }
        let average_status =
            recent_statuses.iter().filter(|b| **b).count() as f32 / recent_statuses.len() as f32;
        if recent_statuses.len() == NUM_STATUSES {
            let range = (0.95 * (target_rps as f32))..(1.05 * (target_rps as f32));
            if ok && recent_rps.iter().all(|x| range.contains(&(*x as f32))) {
                break;
            }
            if !ok && target_rps == 10 {
                break;
            }
        }
        let step = 0.14 * 2.0 * (0.5 - average_status).abs();
        let k = if ok { 1.0 + step } else { 1.0 - step };
        target_rps = 10.max((k * (target_rps as f32)) as u64);
    }
    target_rps
}

#[allow(clippy::unnecessary_wraps)]
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

async fn do_http_request(
    mut conn: TcpStream,
    path: &'static str,
    expected_body: Option<&'static str>,
) -> Result<TcpStream, String> {
    conn.write_all(format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes())
        .await
        .map_err(|e| format!("error writing: {e}"))?;
    let mut buf: FixedBuf<4096> = FixedBuf::default();
    let head = match buf.read_frame_tokio(&mut conn, deframe_http_head).await {
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
    let mut content_length = get_content_length(head)?;
    if let Some(expected_response) = expected_body {
        let deframe_expected_bytes = |b: &[u8]| {
            if content_length <= b.len() {
                Ok((content_length, Some(0..content_length)))
            } else {
                Ok((0, None))
            }
        };
        match buf
            .read_frame_tokio(&mut conn, deframe_expected_bytes)
            .await
        {
            Ok(None) => return Err("error reading: connection closed".to_string()),
            Ok(Some(b)) if b == expected_response.as_bytes() => {}
            Ok(Some(b)) => {
                return Err(format!("unexpected response: {}", escape_and_elide(b, 100)));
            }
            Err(e) => return Err(format!("error reading: {e}")),
        }
    } else {
        content_length -= buf.len();
        buf.clear();
        let writable = buf.writable();
        while content_length > 0 {
            let chunk_size = writable.len().min(content_length);
            let target = &mut writable[..chunk_size];
            conn.read_exact(target)
                .await
                .map_err(|e| format!("error reading: {e}"))?;
            content_length -= chunk_size;
        }
    }
    Ok(conn)
}

static MEDIUM_RESPONSE: [u8; 16384] = [b'M'; 16384];
static LARGE_RESPONSE: [u8; 512 * 1024] = [b'L'; 512 * 1024];

fn main() {
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |req: Request| match req.url.path.as_str() {
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
    let limit = (0.99, Duration::from_millis(100));
    let small_rps = measure_tcp_rps(
        addr,
        0.1,
        limit,
        Box::new(move |conn| Box::pin(do_http_request(conn, "/small", Some("small_response1")))),
    );
    dbg! {small_rps};
    // let medium_rps = measure_tcp_rps(addr, 0.1, &limits, move |conn| {
    //     do_http_request(conn, "/medium", None)
    // });
    // dbg! {medium_rps};
    // let large_rps = measure_tcp_rps(
    //     addr,
    //     0.1,
    //     &vec![(0.99, Duration::from_millis(200))],
    //     move |conn| do_http_request(conn, "/large", None),
    // );
    // dbg! {large_rps};
    // println!("small_rps = {small_rps}, medium_rps = {medium_rps}, large_rps = {large_rps}");

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
