//! Beatrice
//! ========
//! [![crates.io version](https://img.shields.io/crates/v/beatrice.svg)](https://crates.io/crates/beatrice)
//! [![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/beatrice-rs/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
//! [![unsafe forbidden](https://raw.githubusercontent.com/mleonhard/beatrice-rs/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
//! [![pipeline status](https://github.com/mleonhard/beatrice-rs/workflows/CI/badge.svg)](https://github.com/mleonhard/beatrice-rs/actions)
//!
//! A modular HTTP server library in Rust.
//!
//! # Features
//! - `forbid(unsafe_code)`
//! - Threaded request handlers:<br>
//!   `FnOnce(Request) -> Response + 'static + Clone + Send + Sync`
//! - Uses async code for excellent performance under load
//! - JSON
//! - Saves large request bodies to temp files
//! - Sends 100-Continue
//! - Limits number of threads and connections
//! - Modular: use pieces of the library, make async handlers, roll your own logging, etc.
//! - No macros or complicated type params
//! - Good test coverage (??%) - TODO: Update.
//!
//! # Limitations
//! - New, not proven in production.
//! - Does not yet support:
//!   - request timeouts
//!   - chunked transfer-encoding for streaming uploads
//!   - gzip
//!   - brotli
//!   - TLS
//!   - automatically getting TLS certs via ACME
//!   - Denial-of-Service mitigation: source throttling, minimum throughput
//!   - Incomplete functional test suite
//!   - Missing load tests
//!   - Disk space usage limits
//!
//! # Examples
//! Complete example: [`examples/demo.rs`](examples/demo.rs).
//!
//! Simple example:
//! ```rust
//! use beatrice::{
//!     print_log_response,
//!     run_http_server,
//!     socket_addr_127_0_0_1,
//!     Request,
//!     Response
//! };
//! use serde::Deserialize;
//! use serde_json::json;
//! use std::sync::Arc;
//!
//! struct State {}
//!
//! fn hello(_state: Arc<State>, req: Request) -> Result<Response, Response> {
//!     #[derive(Deserialize)]
//!     struct Input {
//!         name: String,
//!     }
//!     let input: Input = req.json()?;
//!     Ok(Response::json(
//!         200,
//!         json!({
//!             "message": format!("Hello, {}!", input.name)
//!         }),
//!     )
//!     .unwrap())
//! }
//!
//! fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Response> {
//!     match (req.method(), req.url().path(), req.content_type()) {
//!         ("GET", "/ping", _) => Ok(Response::text(200, "ok")),
//!         ("POST", "/hello", _) => hello(state, req),
//!         _ => Ok(Response::text(404, "Not found")),
//!     }
//! }
//!
//! let state = Arc::new(State {});
//! let request_handler = move |req: Request| {
//!     print_log_response(
//!         req.method().to_string(),
//!         req.url().clone(),
//!         handle_req(state, req),
//!     )
//! };
//! let listen_addr = socket_addr_127_0_0_1(8000);
//! let num_threads = 10;
//! let max_conns = 1000;
//! let max_vec_body_len = 64 * 1024;
//! # //#[allow(clippy::)]
//! # if false {
//! run_http_server(
//!     listen_addr,
//!     num_threads,
//!     max_conns,
//!     max_vec_body_len,
//!     request_handler,
//! )
//! .unwrap();
//! # }
//! ```
//! # Cargo Geiger Safety Report
//! # Alternatives
//!
//! |                     |    |    |     |    |    |    |    |    |     |    |
//! |---------------------|----|----|-----|----|----|----|----|----|-----|----|
//! |  | beatrice | [rouille](https://crates.io/crates/rouille) | [trillium](https://crates.io/crates/trillium) | [tide](https://crates.io/crates/tide) | [axum](https://crates.io/crates/axum) | [poem](https://crates.io/crates/poem) | [warp](https://crates.io/crates/warp) | [thruster](https://crates.io/crates/thruster) | [rocket](https://crates.io/crates/rocket) | [gotham](https://crates.io/crates/gotham) |
//! | Well-tested         | ❓ | ❌ | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Blocking handlers   | 🟢 | 🟢 | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Async handlers      | ❌ | ❌ | 🟢 | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | 100-continue        | 🟢 | 🟢 | 🟢 | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Thread limit        | 🟢 | ❌ | 🟢 | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | Connection limit    | 🟢 | ❌ | ❌ | ❓ | ❓ | ❌ | ❓ | ❓ | ❓ | ❓ |
//! | Caches payloads     | 🟢 | ❌ | ❌ | ❓ | ❓ | [❌](https://github.com/poem-web/poem/issues/75) | ❓ | ❓ | ❓ | ❓ |
//! | Request timeouts    | ❌ | ❌ | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Custom logging      | 🟢 | 🟢 | 🟢 | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Contains no unsafe  | 🟢 | 🟢 | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | No unsafe deps      | ❌ | ❌ | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | age (years)         | 0  | 6  | 1   | 3  | 0  | 1  | ❓ | ❓ | ❓ | 5 |
//! | TLS                 | ❌ | ❌ | 🟢 | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | ACME certs          | ❌ | ❌ | ❌ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | SSE                 | ❌ | ❌ | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | Websockets          | ❌ | 🟢 | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | Streaming response: |    |    |     |    |    |    |    |    |     | ❓ |
//! | - impl `AsyncRead`  | ❌ | ❌ | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | - `AsyncWrite`      | ❌ | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | - impl `Read`       | ❌ | 🟢 | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | - channel           | ❌ | ❌ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ | ❓ |
//! | Custom routing      | 🟢 | 🟢 | ❓ | ❓ | ❓ | ❌ | ❓ | ❓ | ❓ | ❓ |
//! | Usable sans macros  | 🟢 | 🟢 | ❓ | ❓ | ❓ | ❌ | ❓ | ❓ | ❓ | ❓ |
//! | Shutdown for tests  | ❓ | ❓ | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | Graceful shutdown   | ❓ | ❓ | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❓ | ❓ |
//! | Rust stable         | ❓ | ❓ | ❓ | ❓ | ❓ | 🟢 | ❓ | ❓ | ❌ | ❓ |
//!
//! - [`tide`](https://crates.io/crates/tide)
//!   - Popular
//!   - Does not support uploads (100-Continue): <https://github.com/http-rs/tide/issues/878>
//! - [`actix-web`](https://crates.io/crates/actix)
//!   - Very popular
//!   - Macros
//!   - Contains generous amounts of `unsafe` code
//! - [`rocket`](https://crates.io/crates/rocket)
//!   - Popular
//!   - Macros
//!   - Contains generous amounts of `unsafe` code
//! - [`rouille`](https://crates.io/crates/rouille)
//!   - Popular
//!   - Blocking handlers
//!   - [Uses an unbounded threadpool](https://github.com/tiny-http/tiny-http/issues/221)
//!     and [stops serving after failing once to spawn a thread](https://github.com/tiny-http/tiny-http/issues/220).
//! - TODO: Add others from <https://www.arewewebyet.org/topics/frameworks/>
//!
//! # Changelog
//! - v0.1.0 - First published version
//!
//! # TO DO
//! - Fix limitations above
//!
//! # Release Process
//! 1. Edit `Cargo.toml` and bump version number.
//! 1. Run `./release.sh`
#![forbid(unsafe_code)]
mod accept;
mod body;
mod content_type;
mod head;
mod http_conn;
mod http_error;
mod request;
mod response;
mod token_set;
mod util;

pub use crate::accept::{
    socket_addr_127_0_0_1, socket_addr_127_0_0_1_any_port, socket_addr_all_interfaces,
};
pub use crate::body::{Body, BodyAsyncReader, BodyReader};
pub use crate::content_type::ContentType;
pub use crate::http_conn::HttpConn;
pub use crate::request::Request;
pub use crate::response::Response;

/// To use this module, enable cargo feature `"internals"`.
#[cfg(feature = "internals")]
pub mod internals {
    pub use crate::accept::*;
    pub use crate::body::*;
    pub use crate::content_type::*;
    pub use crate::head::*;
    pub use crate::http_conn::*;
    pub use crate::http_error::*;
    pub use crate::request::*;
    pub use crate::response::*;
    pub use crate::token_set::*;
    pub use crate::util::*;
}

use crate::accept::accept_loop;
use crate::http_conn::handle_http_conn;
use crate::token_set::TokenSet;
use async_net::TcpListener;
use permit::Permit;
use std::net::SocketAddr;
use std::sync::Arc;
use temp_dir::TempDir;
use url::Url;

#[allow(clippy::module_name_repetitions)]
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn print_log_response(
    method: String,
    url: Url,
    result: Result<Response, Response>,
) -> Response {
    let response = result.unwrap_or_else(|e| e);
    println!(
        "{} {} {} => {} len={}",
        if response.code() / 100 == 5 {
            "ERROR"
        } else {
            "INFO"
        },
        method,
        url.path(),
        response.code(),
        response.body().len(),
    );
    response
}

/// Run an HTTP server, listening for connections on `listen_addr`.
///
/// When the server is handling `max_conns` connections,
/// it waits for a connection to drop before accepting new ones.
/// Each connection uses a file handle.
/// Some processes run with a limit on the number of file handles.
/// The kernel also has a limit.
///
/// Creates `num_threads` threads.
/// Creates a safina-timer thread,
/// one async executor thread,
/// and one blocking threadpool thread.
/// When `num_threads` is greater than three,
/// we distribute extra threads between the executor and the threadpool.
///
/// Automatically receives requesst bodies up to 64 KiB.
///
/// If your `request_handler` must handle larger uploads,
/// it should call `Request::body().is_pending()`
/// and return `Response::GetBodyAndReprocess(..)`.
/// Then the server will save the request body to a temporary file
/// and call `request_handler` again.
///
/// # Errors
/// Returns an error when it fails to bind to the `listen_addr` or fails to initially start threads.
///
/// # Panics
/// Panics when `max_conns` is zero.
///
/// Panics when `num_threads` is less than 3.
pub fn run_http_server<F>(
    listen_addr: SocketAddr,
    num_threads: usize,
    max_conns: usize,
    max_vec_body_len: usize,
    request_handler: F,
) -> Result<(), std::io::Error>
where
    F: FnOnce(Request) -> Response + 'static + Clone + Send + Sync,
{
    assert!(max_conns > 0, "max_conns is zero");
    let (num_async, num_blocking) = match num_threads {
        n if n < 3 => panic!("num_threads is less than 3"),
        n if n < 10 => (1, n - 1),
        n if n < 20 => (2, n - 2),
        n if n < 30 => (3, n - 3),
        n => (4, n - 4),
    };
    safina_timer::start_timer_thread();
    let executor = safina_executor::Executor::new(num_async, num_blocking)?;
    executor.block_on(async move {
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let async_request_handler = |req: Request| async move {
            let request_handler_clone = request_handler.clone();
            safina_executor::schedule_blocking(move || request_handler_clone(req))
                .await
                .unwrap_or_else(|_| Response::text(500, "Server error"))
        };
        let conn_handler = move |permit, token, stream: async_net::TcpStream, addr| {
            let http_conn = HttpConn::new(addr, stream);
            let body_dir = temp_dir.path().to_path_buf();
            safina_executor::spawn(handle_http_conn(
                permit,
                token,
                http_conn,
                body_dir,
                max_vec_body_len,
                async_request_handler,
            ));
        };
        safina_executor::block_on(async move {
            let listener = TcpListener::bind(listen_addr).await?;
            let token_set = TokenSet::new(max_conns);
            accept_loop(Permit::new(), listener, token_set, conn_handler).await;
            Ok(())
        })
    })
}
