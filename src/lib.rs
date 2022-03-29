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
//!     socket_addr_127_0_0_1,
//!     HttpServerBuilder,
//!     Request,
//!     Response
//! };
//! use beatrice::reexport::{safina_executor, safina_timer};
//! use serde::Deserialize;
//! use serde_json::json;
//! use std::sync::Arc;
//! use temp_dir::TempDir;
//!
//! struct State {}
//!
//! fn hello(_state: Arc<State>, req: Request) -> Result<Response, Response> {
//!     #[derive(Deserialize)]
//!     struct Input {
//!         name: String,
//!     }
//!     let input: Input = req.json()?;
//!     Ok(Response::json(200, json!({"message": format!("Hello, {}!", input.name)}))
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
//! let cache_dir = TempDir::new().unwrap();
//! safina_timer::start_timer_thread();
//! let executor = safina_executor::Executor::new(1, 9).unwrap();
//! # let permit = permit::Permit::new();
//! # let server_permit = permit.new_sub();
//! # std::thread::spawn(move || {
//! #     std::thread::sleep(std::time::Duration::from_millis(100));
//! #     drop(permit);
//! # });
//! executor.block_on(
//!     HttpServerBuilder::new()
//! #       .permit(server_permit)
//!         .listen_addr(socket_addr_127_0_0_1(8000))
//!         .max_conns(1000)
//!         .small_body_len(64 * 1024)
//!         .receive_large_bodies(cache_dir.path())
//!         .spawn_and_join(request_handler)
//! ).unwrap();
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
//! - Support [HEAD](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/HEAD)
//!   responses that have Content-Length set and no body.
//!
//! # Release Process
//! 1. Edit `Cargo.toml` and bump version number.
//! 1. Run `./release.sh`
#![forbid(unsafe_code)]
mod accept;
mod ascii_string;
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
pub use crate::ascii_string::AsciiString;
pub use crate::body::{Body, BodyAsyncReader, BodyReader};
pub use crate::content_type::ContentType;
pub use crate::http_conn::HttpConn;
pub use crate::request::Request;
pub use crate::response::Response;

pub mod reexport {
    pub use permit;
    pub use safina_executor;
    pub use safina_sync;
    pub use safina_timer;
}

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
use std::path::PathBuf;
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

/// Builds an HTTP server.
pub struct HttpServerBuilder {
    opt_cache_dir: Option<PathBuf>,
    listen_addr: SocketAddr,
    max_conns: usize,
    small_body_len: usize,
    permit: Permit,
}
impl HttpServerBuilder {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            opt_cache_dir: None,
            listen_addr: socket_addr_127_0_0_1_any_port(),
            max_conns: 100,
            small_body_len: 64 * 1024,
            permit: Permit::new(),
        }
    }

    #[must_use]
    pub fn listen_addr(mut self, addr: SocketAddr) -> Self {
        self.listen_addr = addr;
        self
    }

    /// Sets the maximum number of connections to handle at one time.
    ///
    /// When the server is handling the maximum number of connections,
    /// it waits for a connection to drop before accepting new ones.
    ///
    /// Each connection uses a file handle.
    /// Some processes run with a limit on the number of file handles.
    /// The OS kernel also has a limit for all processes combined.
    ///
    /// # Panics
    /// Panics when `n` is zero.
    #[must_use]
    pub fn max_conns(mut self, n: usize) -> Self {
        assert!(n > 0, "refusing to set max_conns to zero");
        self.max_conns = n;
        self
    }

    /// Save large request bodies to this directory.
    ///
    /// If you do not call this method, the server will refuse all
    /// requests with bodies larger than `small_body_len` with `413 Payload Too Large`.
    /// It will also refuse all bodies with unknown length.
    ///
    /// # Example
    /// ```
    /// use std::io::Read;
    /// use beatrice::{HttpServerBuilder, Request, Response};
    /// use beatrice::reexport::{safina_executor, safina_timer};
    ///
    /// let cache_dir = temp_dir::TempDir::new().unwrap();
    /// let handler = move |req: Request| {
    ///     if req.body().is_pending() {
    ///         return Response::GetBodyAndReprocess(1024 * 1024, req);
    ///     }
    ///     let len = req.body().reader().unwrap().bytes().count();
    ///     Response::text(200, format!("body len={}", len))
    /// };
    /// # let permit = permit::Permit::new();
    /// # let server_permit = permit.new_sub();
    /// # std::thread::spawn(move || {
    /// #     std::thread::sleep(std::time::Duration::from_millis(100));
    /// #     drop(permit);
    /// # });
    /// safina_timer::start_timer_thread();
    /// safina_executor::Executor::default().block_on(
    ///     HttpServerBuilder::new()
    /// #       .permit(server_permit)
    ///         .receive_large_bodies(cache_dir.path())
    ///         .spawn_and_join(handler)
    /// ).unwrap();
    /// ```
    #[must_use]
    pub fn receive_large_bodies(mut self, cache_dir: &std::path::Path) -> Self {
        self.opt_cache_dir = Some(cache_dir.to_path_buf());
        self
    }

    /// Automatically receive request bodies up to length `n`,
    /// saving them in memory.
    ///
    /// The default value is 64 KiB.
    ///
    /// Reject larger requests with `413 Payload Too Large`.
    /// See [`receive_large_bodies`](ServerBuilder::receive_large_bodies).
    ///
    /// You can estimate the server memory usage with:
    /// `small_body_len * max_conns`.
    /// Using the default settings: 64 KiB * 100 connections => 6.4 MiB.
    #[must_use]
    pub fn small_body_len(mut self, n: usize) -> Self {
        self.small_body_len = n;
        self
    }

    /// Sets the permit used by the server.
    ///
    /// Revoke the permit to make the server gracefully shut down.
    ///
    /// # Example
    /// ```
    /// use std::net::SocketAddr;
    /// use permit::Permit;
    /// use beatrice::{Response, HttpServerBuilder};
    /// use beatrice::reexport::{safina_executor, safina_timer};
    /// # fn do_some_requests(addr: SocketAddr) -> Result<(),()> { Ok(()) }
    ///
    /// safina_timer::start_timer_thread();
    /// let executor = safina_executor::Executor::default();
    /// let permit = Permit::new();
    /// let (addr, stopped_receiver) = executor.block_on(
    ///     HttpServerBuilder::new()
    ///         .permit(permit.new_sub())
    ///         .spawn(move |req| Response::text(200, "yo"))
    /// ).unwrap();
    /// do_some_requests(addr).unwrap();
    /// drop(permit); // Tell server to shut down.
    /// stopped_receiver.recv(); // Wait for server to stop.
    /// ```
    #[must_use]
    pub fn permit(mut self, p: Permit) -> Self {
        self.permit = p;
        self
    }

    /// Spawns the server task.
    ///
    /// Returns `(addr, stopped_receiver)`.
    /// The server is listening on `addr`.
    /// After the server gracefully shuts down, it sends a message on `stopped_receiver`.
    ///
    /// # Errors
    /// Returns an error when it fails to bind to the [`listen_addr`](ServerBuilder::listen_addr).
    pub async fn spawn<F>(
        self,
        request_handler: F,
    ) -> Result<(SocketAddr, reexport::safina_sync::Receiver<()>), std::io::Error>
    where
        F: FnOnce(Request) -> Response + 'static + Clone + Send + Sync,
    {
        let async_request_handler = |req: Request| async move {
            let request_handler_clone = request_handler.clone();
            safina_executor::schedule_blocking(move || request_handler_clone(req))
                .await
                .unwrap_or_else(|_| Response::text(500, "Server error"))
        };
        let conn_handler = move |permit, token, stream: async_net::TcpStream, addr| {
            let http_conn = HttpConn::new(addr, stream);
            safina_executor::spawn(handle_http_conn(
                permit,
                token,
                http_conn,
                self.opt_cache_dir,
                self.small_body_len,
                async_request_handler,
            ));
        };
        let listener = TcpListener::bind(self.listen_addr).await?;
        let addr = listener.local_addr()?;
        let token_set = TokenSet::new(self.max_conns);
        let (sender, receiver) = safina_sync::oneshot();
        safina_executor::spawn(async move {
            accept_loop(self.permit, listener, token_set, conn_handler).await;
            // TODO: Wait for connection tokens to return.
            let _ignored = sender.send(());
        });
        Ok((addr, receiver))
    }

    /// Spawns the server task and waits for it to shutdown gracefully.
    ///
    /// # Errors
    /// Returns an error when it fails to bind to the [`listen_addr`](ServerBuilder::listen_addr).
    pub async fn spawn_and_join<F>(self, request_handler: F) -> Result<(), std::io::Error>
    where
        F: FnOnce(Request) -> Response + 'static + Clone + Send + Sync,
    {
        let (_addr, mut stopped_receiver) = self.spawn(request_handler).await?;
        let _ignored = stopped_receiver.async_recv().await;
        Ok(())
    }
}
