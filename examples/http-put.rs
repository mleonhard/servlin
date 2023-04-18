//! HTTP PUT Server Example
//! =======================
//!
//! Start the server:
//! ```
//! cargo run --package servlin --example http-put
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.04s
//!      Running `target/debug/examples/http-put`
//! Access the server at http://127.0.0.1:8000/upload
//! INFO PUT /upload => 200 len=44
//! INFO PUT /upload => 200 len=44
//! ^C
//! ```
//!
//! Make requests to it:
//! ```
//! $ echo -n abc >abc.txt                                       
//! $ curl http://127.0.0.1:8000/upload --upload-file abc.txt    
//! Upload received, body_len=3, upload_count=1
//! $ echo -n 12345 >12345.txt                                   
//! $ curl http://127.0.0.1:8000/upload --upload-file 12345.txt
//! Upload received, body_len=5, upload_count=2
//! ```
#![forbid(unsafe_code)]
use servlin::log::{log_response, set_global_logger, LogFileWriter};
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

fn put(state: &Arc<State>, req: &Request) -> Result<Response, Error> {
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

fn handle_req(state: &Arc<State>, req: &Request) -> Result<Response, Error> {
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
    let request_handler = move |req: Request| log_response(&req, handle_req(&state, &req)).unwrap();
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
