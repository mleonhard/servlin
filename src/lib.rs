//! Servlin
//! ========
//! [![crates.io version](https://img.shields.io/crates/v/servlin.svg)](https://crates.io/crates/servlin)
//! [![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/servlin/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
//! [![unsafe forbidden](https://raw.githubusercontent.com/mleonhard/servlin/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
//! [![pipeline status](https://github.com/mleonhard/servlin/workflows/CI/badge.svg)](https://github.com/mleonhard/servlin/actions)
//!
//! A modular HTTP server library in Rust.
//!
//! # Features
//! - `forbid(unsafe_code)`
//! - Threaded request handlers:<br>
//!   `FnOnce(Request) -> Response + 'static + Clone + Send + Sync`
//! - Uses async code internally for excellent performance under load
//! - JSON
//! - Server-Sent Events (SSE)
//! - Saves large request bodies to temp files
//! - Sends 100-Continue
//! - Limits number of threads and connections
//! - Modular: roll your own logging, write custom versions of internal methods, etc.
//! - No macros or complicated type params
//! - Good test coverage (63%)
//!
//! # Limitations
//! - New, not proven in production.
//! - To do:
//!   - Request timeouts
//!   - `chunked` transfer-encoding for request bodies
//!   - gzip
//!   - brotli
//!   - TLS
//!   - automatically getting TLS certs via ACME
//!   - Drop idle connections when approaching connection limit.
//!   - Denial-of-Service mitigation: source throttling, minimum throughput
//!   - Complete functional test suite
//!   - Missing load tests
//!   - Disk space usage limits
//!
//! # Examples
//! Complete examples: [`examples/`](https://github.com/mleonhard/servlin/tree/main/examples).
//!
//! Simple example:
//! ```rust
//! use serde::Deserialize;
//! use serde_json::json;
//! use servlin::{
//!     socket_addr_127_0_0_1,
//!     Error,
//!     HttpServerBuilder,
//!     Request,
//!     Response
//! };
//! use servlin::log::log_request_and_response;
//! use std::sync::Arc;
//! use temp_dir::TempDir;
//!
//! struct State {}
//!
//! fn hello(_state: Arc<State>, req: Request) -> Result<Response, Error> {
//!     #[derive(Deserialize)]
//!     struct Input {
//!         name: String,
//!     }
//!     let input: Input = req.json()?;
//!     Ok(Response::json(200, json!({"message": format!("Hello, {}!", input.name)}))?)
//! }
//!
//! fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
//!     match (req.method(), req.url().path()) {
//!         ("GET", "/ping") => Ok(Response::text(200, "ok")),
//!         ("POST", "/hello") => hello(state, req),
//!         _ => Ok(Response::text(404, "Not found")),
//!     }
//! }
//!
//! let state = Arc::new(State {});
//! let request_handler = move |req: Request| {
//!     log_request_and_response(req, |req| handle_req(state, req)).unwrap()
//! };
//! let cache_dir = TempDir::new().unwrap();
//! safina::timer::start_timer_thread();
//! let executor = safina::executor::Executor::new(1, 9).unwrap();
//! # let permit = permit::Permit::new();
//! # let server_permit = permit.new_sub();
//! # std::thread::spawn(move || {
//! #     std::thread::sleep(std::time::Duration::from_millis(100));
//! #     drop(permit);
//! # });
//! executor.block_on(
//!     HttpServerBuilder::new()
//! #       .permit(server_permit)
//!         .listen_addr(socket_addr_127_0_0_1(8271))
//!         .max_conns(1000)
//!         .small_body_len(64 * 1024)
//!         .receive_large_bodies(cache_dir.path())
//!         .spawn_and_join(request_handler)
//! ).unwrap();
//! ```
//! # Cargo Geiger Safety Report
//! # Alternatives
//! See [rust-webserver-comparison.md](https://github.com/mleonhard/servlin/blob/main/rust-webserver-comparison.md).
//!
//! # Changelog
//! - v0.6.1 2024-11-03 - Implement `Into<TagList>` for arrays.
//! - v0.6.0 2024-11-02
//!    - Remove `servlin::reexports` module.
//!    - Use `safina` v0.6.0.
//! - v0.5.1 2024-10-26 - Remove dependency on `once_cell`.
//! - v0.5.0 2024-10-21 - Remove `LogFileWriterBuilder`.
//! - v0.4.3 - Implement `From<Cow<'_, str>>` and `From<&Path>` for `TagValue`.
//! - v0.4.2 - Implement `Seek` for `BodyReader`.
//! - v0.4.1
//!   - Add `Request::opt_json`.
//!   - Implement `From<LoggerStoppedError>` for `Error`.
//! - v0.4.0
//!   - Changed `Response::json` to return `Result<Response, Error>`.
//!   - Changed `log_request_and_response` to return `Result`.
//!   - Added `Response::unprocessable_entity_422`.
//! - v0.3.2 - Fix bug in `Response::include_dir` redirects.
//! - v0.3.1
//!   - Add `Response::redirect_301`
//!   - `Response::include_dir` to redirect from `/somedir` to `/somedir/` so relative URLs will work.
//! - v0.3.0 - Changed `Response::include_dir` to take `&Request` and look for `index.html` in dirs.
//! - v0.2.0
//!   - Added:
//!     - `log_request_and_response` and other logging tooling
//!     - `Response::ok_200()`
//!     - `Response::unauthorized_401()`
//!     - `Response::forbidden_403()`
//!     - `Response::internal_server_errror_500()`
//!     - `Response::not_implemented_501()`
//!     - `Response::service_unavailable_503()`
//!     - `EventSender::is_connected()`
//!     - `PORT_env()`
//!   - Removed `print_log_response` and `RequestBody::length_is_known`
//!   - Changed `RequestBody::len` and `is_empty` to return `Option`.
//!   - Bugfixes
//! - v0.1.1 - Add `EventSender::unconnected`.
//! - v0.1.0 - Rename library to Servlin.
//!
//! # TO DO
//! - Fix limitations above
//! - Support [HEAD](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/HEAD)
//!   responses that have Content-Length set and no body.
//! - Add a server-wide limit on upload body size.
//! - Limit disk usage for caching uploads.
//! - Update `rust-webserver-comparison.md`
//!   - Add missing data
//!   - Add other servers from <https://www.arewewebyet.org/topics/frameworks/>
//!   - Rearrange
//!   - Generate geiger reports for each web server
#![forbid(unsafe_code)]
mod accept;
mod ascii_string;
mod body_async_reader;
mod body_reader;
mod content_type;
mod cookie;
mod error;
mod event;
mod head;
mod headers;
mod http_conn;
mod http_error;
pub mod log;
mod rand;
mod request;
mod request_body;
mod response;
mod response_body;
mod time;
mod token_set;
mod util;

pub use crate::accept::{
    socket_addr_127_0_0_1, socket_addr_127_0_0_1_any_port, socket_addr_all_interfaces, PORT_env,
};
pub use crate::ascii_string::AsciiString;
pub use crate::body_async_reader::BodyAsyncReader;
pub use crate::body_reader::BodyReader;
pub use crate::content_type::ContentType;
pub use crate::cookie::{Cookie, SameSite};
pub use crate::error::Error;
pub use crate::event::{Event, EventSender};
pub use crate::headers::{Header, HeaderList};
pub use crate::http_conn::HttpConn;
pub use crate::request::Request;
pub use crate::request_body::RequestBody;
pub use crate::response::Response;
pub use crate::response_body::ResponseBody;

/// This part of the library is not covered by the semver guarantees.
/// If you use these in your program, a minor version upgrade could break your build.
///
/// If you use these items in a published library,
/// your library should depend on a specific version of this library.
pub mod internal {
    pub use crate::accept::*;
    pub use crate::body_async_reader::*;
    pub use crate::body_reader::*;
    pub use crate::content_type::*;
    pub use crate::cookie::*;
    pub use crate::event::*;
    pub use crate::head::*;
    pub use crate::headers::*;
    pub use crate::http_conn::*;
    pub use crate::http_error::*;
    pub use crate::request::*;
    pub use crate::request_body::*;
    pub use crate::response::*;
    pub use crate::response_body::*;
    pub use crate::time::*;
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

/// Builds an HTTP server.
pub struct HttpServerBuilder {
    opt_cache_dir: Option<PathBuf>,
    listen_addr: SocketAddr,
    max_conns: usize,
    small_body_len: usize,
    permit: Permit,
}
impl HttpServerBuilder {
    /// Makes a new builder these default settings:
    /// - Listens on 127.0.0.1
    /// - Picks a random port
    /// - 100 max connections
    /// - 64 KiB small body length
    /// - no cache dir, server rejects large request bodies
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
    /// use servlin::{HttpServerBuilder, Request, Response};
    /// use std::io::Read;
    ///
    /// let cache_dir = temp_dir::TempDir::new().unwrap();
    /// let handler = move |req: Request| {
    ///     if req.body.is_pending() {
    ///         return Response::get_body_and_reprocess(1024 * 1024);
    ///     }
    ///     let len = req.body.reader().unwrap().bytes().count();
    ///     Response::text(200, format!("body len={}", len))
    /// };
    /// # let permit = permit::Permit::new();
    /// # let server_permit = permit.new_sub();
    /// # std::thread::spawn(move || {
    /// #     std::thread::sleep(std::time::Duration::from_millis(100));
    /// #     drop(permit);
    /// # });
    /// safina::timer::start_timer_thread();
    /// safina::executor::Executor::new(1, 1)
    ///   .unwrap()
    ///   .block_on(
    ///     HttpServerBuilder::new()
    /// #     .permit(server_permit)
    ///       .receive_large_bodies(cache_dir.path())
    ///       .spawn_and_join(handler)
    ///   )
    ///   .unwrap();
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
    /// use std::sync::Arc;
    /// use permit::Permit;    
    /// use safina::executor::Executor;
    /// use servlin::{Response, HttpServerBuilder};
    /// # fn do_some_requests(addr: SocketAddr) -> Result<(),()> { Ok(()) }
    ///
    /// safina::timer::start_timer_thread();
    /// let executor: Arc<Executor> = Arc::default();
    /// let permit = Permit::new();
    /// let (addr, stopped_receiver) = executor.block_on(
    ///     HttpServerBuilder::new()
    ///         .permit(permit.new_sub())
    ///         .spawn(move |req| Response::text(200, "yo"))
    /// ).unwrap();
    /// do_some_requests(addr).unwrap();
    /// drop(permit); // Tell server to shut down.
    /// stopped_receiver.recv().unwrap(); // Wait for server to stop.
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
    ) -> Result<(SocketAddr, safina::sync::Receiver<()>), std::io::Error>
    where
        F: FnOnce(Request) -> Response + 'static + Clone + Send + Sync,
    {
        let async_request_handler = |req: Request| async move {
            let request_handler_clone = request_handler.clone();
            safina::executor::schedule_blocking(move || request_handler_clone(req))
                .await
                .unwrap_or_else(|_| Response::text(500, "Server error"))
        };
        let conn_handler = move |permit, token, stream: async_net::TcpStream, addr| {
            let http_conn = HttpConn::new(addr, stream);
            safina::executor::spawn(handle_http_conn(
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
        let (sender, receiver) = safina::sync::oneshot();
        safina::executor::spawn(async move {
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
