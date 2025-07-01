//! Minimal Server Example
//! =================
//!
//! Start the server:
//! ```
//! $ cargo run --package servlin --example minimal
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
use safina::executor::Executor;
use servlin::{HttpServerBuilder, Request, Response, socket_addr_127_0_0_1};
use std::sync::Arc;

pub fn main() {
    safina::timer::start_timer_thread();
    let executor: Arc<Executor> = Arc::default();
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .spawn_and_join(|_req: Request| Response::not_found_404()),
        )
        .unwrap();
}
