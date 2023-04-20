//! HTTP PUT Server Example
//! =======================
//!
//! Start the server:
//! ```
//! $ cargo run --package servlin --example http-put
//!    Compiling servlin v0.1.2 (/Users/user/servlin)
//!     Finished dev [unoptimized + debuginfo] target(s) in 2.56s
//!      Running `target/debug/examples/http-put`
//! Access the server at http://127.0.0.1:8000/upload
//! ^C
//! $ cat log.20230420T172728Z-0
//! {"time":"2023-04-20T17:27:28Z","level":"info","msg":"Starting log writer","time_ns":1682011648182924000}
//! {"time":"2023-04-20T17:27:37Z","level":"info","code":200,"response_body_len":44,"http_method":"PUT","path":"/upload","request_id":13447028046809572982,"time_ns":1682011657080608000}
//! {"time":"2023-04-20T17:27:43Z","level":"info","code":200,"response_body_len":44,"http_method":"PUT","path":"/upload","request_id":10015697617888059338,"time_ns":1682011663410536000}
//! $
//! ```
//!
//! Make requests to it:
//! ```
//! $ cargo run --package servlin --features urlencoded --example html_form
//! $ echo -n abc >abc.txt
//! $ curl http://127.0.0.1:8000/upload --upload-file abc.txt
//! Upload received, body_len=3, upload_count=1
//! $ echo -n 12345 >12345.txt
//! $ curl http://127.0.0.1:8000/upload --upload-file 12345.txt
//! Upload received, body_len=5, upload_count=2
//! $
//! ```
#![forbid(unsafe_code)]
use servlin::log::{log_request_and_response, set_global_logger, LogFileWriter};
use servlin::reexport::{safina_executor, safina_timer};
use servlin::{socket_addr_127_0_0_1, Error, HttpServerBuilder, Request, Response};
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use temp_dir::TempDir;

pub struct State {
    upload_count: AtomicUsize,
}
impl State {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        State {
            upload_count: AtomicUsize::new(0),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn put(state: Arc<State>, req: Request) -> Result<Response, Error> {
    if req.body.is_pending() {
        return Ok(Response::get_body_and_reprocess(1024 * 1024));
    }
    let body_len = req.body.reader()?.bytes().count();
    state.upload_count.fetch_add(1, Ordering::AcqRel);
    Ok(Response::text(
        200,
        format!(
            "Upload received, body_len={}, upload_count={}\n",
            body_len,
            state.upload_count.load(Ordering::Acquire)
        ),
    ))
}

fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
    match (req.method(), req.url().path()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("PUT", "/upload") => put(state, req),
        (_, "/upload") => Ok(Response::method_not_allowed_405(&["PUT"])),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    println!("Access the server at http://127.0.0.1:8000/upload");
    set_global_logger(
        LogFileWriter::new_builder("log", 100 * 1024 * 1024)
            .start_writer_thread()
            .unwrap(),
    )
    .unwrap();
    safina_timer::start_timer_thread();
    let executor = safina_executor::Executor::default();
    let cache_dir = TempDir::new().unwrap();
    let state = Arc::new(State::new());
    let request_handler =
        move |req: Request| log_request_and_response(req, |req| handle_req(state, req));
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .max_conns(100)
                .small_body_len(64 * 1024)
                .receive_large_bodies(cache_dir.path())
                .spawn_and_join(request_handler),
        )
        .unwrap();
}
