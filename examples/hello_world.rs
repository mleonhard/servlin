//! Minimal Server Example
//! =================
//!
//! Start the server:
//! ```
//! $ cargo run --package servlin --example hello_world
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.04s
//!      Running `target/debug/examples/minimal`
//! ^C
//! ```
//!
//! Make a request to it:
//! ```
//! $ curl -v http://127.0.0.1:8000/
//! *   Trying 127.0.0.1:8000...
//! * Connected to 127.0.0.1 (127.0.0.1) port 8000 (#0)
//! > GET / HTTP/1.1
//! > Host: 127.0.0.1:8000
//! > User-Agent: curl/7.79.1
//! > Accept: */*
//! >
//! * Mark bundle as not supporting multiuse
//! < HTTP/1.1 404 Not Found
//! < content-type: text/plain; charset=UTF-8
//! < content-length: 9
//! <
//! * Connection #0 to host 127.0.0.1 left intact
//! not found
//! $
//! ```
#![forbid(unsafe_code)]
use permit::Permit;
use safina::executor::Executor;
use servlin::{HttpServerBuilder, Response, socket_addr_127_0_0_1};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::Arc;

pub fn main() {
    safina::timer::start_timer_thread();
    let executor: Arc<Executor> = Arc::default();
    let permit = Permit::new();
    safina::timer::start_timer_thread();
    let handler = |_req| Response::text(200, "Hello, World!");
    let (_addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(3000))
                .max_conns(1000)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    Signals::new([SIGTERM, SIGINT]).unwrap().into_iter().next();
}
