Servlin
========
[![crates.io version](https://img.shields.io/crates/v/servlin.svg)](https://crates.io/crates/servlin)
[![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/servlin/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
[![unsafe forbidden](https://raw.githubusercontent.com/mleonhard/servlin/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![pipeline status](https://github.com/mleonhard/servlin/workflows/CI/badge.svg)](https://github.com/mleonhard/servlin/actions)

A modular HTTP server library in Rust.

# Features
- `forbid(unsafe_code)`
- Threaded request handlers:<br>
  `FnOnce(Request) -> Response + 'static + Clone + Send + Sync`
- Uses async code internally for excellent performance under load
- JSON
- Server-Sent Events (SSE)
- Saves large request bodies to temp files
- Sends 100-Continue
- Limits number of threads and connections
- Modular: roll your own logging, write custom versions of internal methods, etc.
- No macros or complicated type params
- Good test coverage (63%)

# Limitations
- New, not proven in production.
- To do:
  - Request timeouts
  - `chunked` transfer-encoding for request bodies
  - gzip
  - brotli
  - TLS
  - automatically getting TLS certs via ACME
  - Drop idle connections when approaching connection limit.
  - Denial-of-Service mitigation: source throttling, minimum throughput
  - Complete functional test suite
  - Missing load tests
  - Disk space usage limits

# Examples
Complete examples: [`examples/`](https://github.com/mleonhard/servlin/tree/main/examples).

Simple example:
```rust
use serde::Deserialize;
use serde_json::json;
use servlin::{
    socket_addr_127_0_0_1,
    Error,
    HttpServerBuilder,
    Request,
    Response
};
use servlin::log::log_request_and_response;
use servlin::reexport::{safina_executor, safina_timer};
use std::sync::Arc;
use temp_dir::TempDir;

struct State {}

fn hello(_state: Arc<State>, req: Request) -> Result<Response, Error> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let input: Input = req.json()?;
    Ok(Response::json(200, json!({"message": format!("Hello, {}!", input.name)}))
    .unwrap())
}

fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
    match (req.method(), req.url().path()) {
        ("GET", "/ping") => Ok(Response::text(200, "ok")),
        ("POST", "/hello") => hello(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

let state = Arc::new(State {});
let request_handler = move |req: Request| {
    log_request_and_response(req, |req| handle_req(state, req)).unwrap()
};
let cache_dir = TempDir::new().unwrap();
safina_timer::start_timer_thread();
let executor = safina_executor::Executor::new(1, 9).unwrap();
executor.block_on(
    HttpServerBuilder::new()
        .listen_addr(socket_addr_127_0_0_1(8271))
        .max_conns(1000)
        .small_body_len(64 * 1024)
        .receive_large_bodies(cache_dir.path())
        .spawn_and_join(request_handler)
).unwrap();
```
# Cargo Geiger Safety Report
# Alternatives
See [rust-webserver-comparison.md](https://github.com/mleonhard/servlin/blob/main/rust-webserver-comparison.md).

# Changelog
- v0.4.1
  - Add `Request::opt_json`.
  - Implement `From<LoggerStoppedError>` for `Error`.
- v0.4.0
  - Changed `Response::json` to return `Result<Response, Error>`.
  - Changed `log_request_and_response` to return `Result`.
  - Added `Response::unprocessable_entity_422`.
- v0.3.2 - Fix bug in `Response::include_dir` redirects.
- v0.3.1
  - Add `Response::redirect_301`
  - `Response::include_dir` to redirect from `/somedir` to `/somedir/` so relative URLs will work.
- v0.3.0 - Changed `Response::include_dir` to take `&Request` and look for `index.html` in dirs.
- v0.2.0
  - Added:
    - `log_request_and_response` and other logging tooling
    - `Response::ok_200()`
    - `Response::unauthorized_401()`
    - `Response::forbidden_403()`
    - `Response::internal_server_errror_500()`
    - `Response::not_implemented_501()`
    - `Response::service_unavailable_503()`
    - `EventSender::is_connected()`
    - `PORT_env()`
  - Removed `print_log_response` and `RequestBody::length_is_known`
  - Changed `RequestBody::len` and `is_empty` to return `Option`.
  - Bugfixes
- v0.1.1 - Add `EventSender::unconnected`.
- v0.1.0 - Rename library to Servlin.

# TO DO
- Fix limitations above
- Support [HEAD](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/HEAD)
  responses that have Content-Length set and no body.
- Add a server-wide limit on upload body size.
- Limit disk usage for caching uploads.
- Update `rust-webserver-comparison.md`
  - Add missing data
  - Add other servers from <https://www.arewewebyet.org/topics/frameworks/>
  - Rearrange
  - Generate geiger reports for each web server

License: MIT OR Apache-2.0
