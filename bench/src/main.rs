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
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::Thread;
use std::time::{Duration, Instant};

fn connect(addr: SocketAddr) -> Result<TcpStream, String> {
    let tcp_stream = TcpStream::connect_timeout(&addr, Duration::from_millis(1000))
        .map_err(|e| format!("connect error: {e}"))?;
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(1000)));
    let _ = tcp_stream.set_write_timeout(Some(Duration::from_millis(1000)));
    Ok(tcp_stream)
}

pub struct ThreadWaker(pub Thread);
impl Drop for ThreadWaker {
    fn drop(&mut self) {
        self.0.unpark();
    }
}

fn measure_once(
    addr: &SocketAddr,
    f: &Arc<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)>,
    opt_conn: Option<TcpStream>,
) -> Result<Option<TcpStream>, String> {
    let mut conn = match opt_conn {
        None => connect(*addr).map_err(|e| format!("error connecting: {e}"))?,
        Some(conn) => conn,
    };
    f(&mut conn)?;
    Ok(Some(conn))
}

fn measure_thread(batch_rx: Receiver<BatchRequest>) {
    let mut opt_conn = None;
    for batch in batch_rx.iter() {
        let mut result = BatchResult::new();
        for time in batch.times {
            let now = Instant::now();
            result.slip_nanos += now.saturating_duration_since(time).as_nanos() as u64;
            sleep_until(time);
            let before = Instant::now();
            match measure_once(&batch.addr, &batch.f, opt_conn.take()) {
                Ok(Some(conn)) => {
                    result.add_duration(before.elapsed());
                    opt_conn = Some(conn);
                }
                Ok(None) => result.add_duration(before.elapsed()),
                Err(e) => result.add_error(e),
            }
        }
        let _ = batch.result_tx.send(result);
    }
}

fn make_measure_thread(batch_rx: Receiver<BatchRequest>) {
    std::thread::Builder::new()
        .spawn(move || measure_thread(batch_rx))
        .map_err(|e| format!("error creating thread: {e}"))
        .unwrap();
}

fn sleep_until(t: Instant) {
    let wait_time = t.saturating_duration_since(Instant::now());
    if !wait_time.is_zero() {
        std::thread::sleep(wait_time);
    }
}

#[derive(Clone)]
struct BatchRequest {
    addr: SocketAddr,
    f: Arc<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)>,
    times: Vec<Instant>,
    result_tx: Sender<BatchResult>,
}
impl BatchRequest {
    pub fn new(
        addr: SocketAddr,
        f: &Arc<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)>,
        result_tx: Sender<BatchResult>,
    ) -> Self {
        Self {
            addr,
            f: Arc::clone(f),
            times: Vec::new(),
            result_tx,
        }
    }
}

fn enqueue_requests(
    addr: SocketAddr,
    f: &Arc<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)>,
    rps: usize,
    start: Instant,
    thread_count: usize,
    batch_tx: &Sender<BatchRequest>,
) -> Receiver<BatchResult> {
    let (result_tx, result_rx) = crossbeam_channel::unbounded();
    let mut batches: Vec<BatchRequest> = (0..thread_count)
        .into_iter()
        .map(|_| BatchRequest::new(addr, &f, result_tx.clone()))
        .collect();
    let num_requests = rps / 10;
    let mut offset = Duration::ZERO;
    let spacing = Duration::from_nanos(1_000_000_000 / rps as u64);
    let batch_len = batches.len();
    for n in 0..num_requests {
        let time = start.add(offset);
        offset += spacing;
        let batch_num = n % batch_len;
        batches[batch_num].times.push(time);
    }
    for batch in batches {
        batch_tx.send(batch).unwrap();
    }
    println!("enqueued={num_requests}");
    result_rx
}

struct BatchResult {
    count: usize,
    errors: usize,
    error_counts: HashMap<String, usize>,
    durations: Vec<Duration>,
    slip_nanos: u64,
}
impl BatchResult {
    pub fn new() -> Self {
        Self {
            count: 0,
            errors: 0,
            error_counts: Default::default(),
            durations: vec![],
            slip_nanos: 0,
        }
    }
    pub fn add_error(&mut self, e: String) {
        self.count += 1;
        self.errors += 1;
        self.error_counts
            .entry(e)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    pub fn add_duration(&mut self, duration: Duration) {
        self.count += 1;
        self.durations.push(duration);
    }
}

struct Summary {
    count: usize,
    errors: f32,
    mean_slip: Duration,
}
fn summarize_results(results: &[BatchResult]) -> Summary {
    let count = results.iter().map(|r| r.count).sum::<usize>();
    let error_count = results.iter().map(|r| r.errors).sum::<usize>();
    let errors = error_count as f32 / count.max(1) as f32;
    let slip_nanos_sum = results.iter().map(|r| r.slip_nanos).sum::<u64>();
    let mean_slip_nanos = slip_nanos_sum / count.max(1) as u64;
    let mean_slip = Duration::from_nanos(mean_slip_nanos);
    Summary {
        count,
        errors,
        mean_slip,
    }
}

fn combine_error_counts(results: Vec<BatchResult>) -> HashMap<String, usize> {
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    for result in results {
        for (e, count) in result.error_counts {
            if let Some(count_sum) = error_counts.get_mut(&e) {
                *count_sum += count;
            } else {
                error_counts.insert(e, count);
            }
        }
    }
    error_counts
}

fn get_proportion(results: &[BatchResult], max_duration: &Duration) -> f32 {
    let ok_count = results
        .iter()
        .map(|r| r.durations.iter().filter(|d| *d < max_duration).count())
        .sum::<usize>();
    let all_count = results.iter().map(|r| r.durations.len()).sum::<usize>();
    ok_count as f32 / all_count.max(1) as f32
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MaxMeasurableRps(usize);

pub fn measure_tcp_rps(
    addr: SocketAddr,
    error_limit: f32,
    time_limits: &Vec<(f32, Duration)>,
    f: impl 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>),
) -> Result<usize, MaxMeasurableRps> {
    let f: Arc<dyn 'static + Sync + Send + (Fn(&mut TcpStream) -> Result<(), String>)> =
        Arc::new(f);
    let (batch_tx, batch_rx) = crossbeam_channel::unbounded();
    let mut num_threads = 10;
    for _ in 0..num_threads {
        make_measure_thread(batch_rx.clone());
    }
    let mut start = Instant::now() + Duration::from_millis(100);
    let mut rps = 10usize;
    let mut prev_prev_result_rx: Receiver<BatchResult> = crossbeam_channel::unbounded().1;
    let mut prev_result_rx: Receiver<BatchResult> = crossbeam_channel::unbounded().1;
    let mut recent_statuses = VecDeque::new();
    let mut recent_rps = VecDeque::new();
    loop {
        let result_rx = enqueue_requests(addr, &f, rps, start, num_threads, &batch_tx);
        // Check results.
        let results: Vec<BatchResult> = prev_prev_result_rx.iter().collect();
        prev_prev_result_rx = prev_result_rx;
        prev_result_rx = result_rx;
        let mut ok = true;
        let summary = summarize_results(&results);
        if Duration::from_millis(100) < summary.mean_slip {
            ok = false;
            let to_add = 1.min(((num_threads as f32) * 1.1) as usize);
            for _ in 0..to_add {
                make_measure_thread(batch_rx.clone());
            }
            num_threads += to_add;
        }
        if error_limit < summary.errors {
            ok = false;
        }
        let mut msg = format!(
            "threads={num_threads} count={} errors={:.3} mean_slip_ms={}",
            summary.count,
            summary.errors,
            summary.mean_slip.as_millis()
        );
        for (min_proportion, max_duration) in time_limits {
            let ok_proportion = get_proportion(&results, &max_duration);
            if ok_proportion < *min_proportion {
                ok = false;
            }
            msg.push_str(&format!(" {max_duration:?}=P{:.3}", 100.0 * ok_proportion))
        }
        // msg.push_str(&format!(" err={:?}", combine_error_counts(results)));
        if ok && start < Instant::now() {
            println!("ok={ok} {msg}");
            return Err(MaxMeasurableRps(rps));
        }
        // Adjust RPS.
        recent_statuses.push_front(ok);
        const NUM_STATUSES: usize = 15;
        while NUM_STATUSES < recent_statuses.len() {
            recent_statuses.pop_back();
        }
        recent_rps.push_front(rps);
        while NUM_STATUSES < recent_rps.len() {
            recent_rps.pop_back();
        }
        let average_status =
            recent_statuses.iter().filter(|b| **b).count() as f32 / recent_statuses.len() as f32;
        if recent_statuses.len() == NUM_STATUSES {
            let range = (0.95 * (rps as f32))..(1.05 * (rps as f32));
            if ok && recent_rps.iter().all(|x| range.contains(&(*x as f32))) {
                break;
            }
            if !ok && rps == 10 {
                break;
            }
        }
        let step = 0.14 * 2.0 * (0.5 - average_status).abs();
        let k = if ok { 1.0 + step } else { 1.0 - step };
        rps = 10.max((k * (rps as f32)) as usize);
        println!("ok={ok} {msg} rps={rps}");
        sleep_until(start);
        start += Duration::from_millis(100);
    }
    Ok(rps)
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
    let limits = vec![(0.99, Duration::from_millis(100))];
    let small_rps = measure_tcp_rps(addr, 0.1, &limits, move |conn| {
        do_http_request(conn, "/small", Some("small_response1"))
    })
    .unwrap();
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
