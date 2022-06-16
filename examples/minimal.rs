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
//! $ curl http://127.0.0.1:8000/   
//! not found
//! ```
#![forbid(unsafe_code)]
use servlin::reexport::{safina_executor, safina_timer};
use servlin::{socket_addr_127_0_0_1, HttpServerBuilder, Request, Response};

pub fn main() {
    safina_timer::start_timer_thread();
    let executor = safina_executor::Executor::default();
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .spawn_and_join(|_req: Request| Response::not_found_404()),
        )
        .unwrap();
}
