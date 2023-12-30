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
```

Metric output format: x/y
    x = unsafe code used by the build
    y = total unsafe code found in the crate

Symbols: 
    🔒  = No `unsafe` usage found, declares #![forbid(unsafe_code)]
    ❓  = No `unsafe` usage found, missing #![forbid(unsafe_code)]
    ☢️  = `unsafe` usage found

Functions  Expressions  Impls  Traits  Methods  Dependency

0/0        0/0          0/0    0/0     0/0      🔒  servlin 0.4.0
0/0        0/4          0/0    0/0     0/2      ❓  ├── async-fs 1.6.0
0/4        0/230        0/40   0/0     0/12     ❓  │   ├── async-lock 2.8.0
0/0        0/116        0/8    0/0     0/0      ❓  │   │   └── event-listener 2.5.3
0/0        0/28         0/4    0/0     0/0      ❓  │   ├── blocking 1.3.1
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── async-channel 1.9.0
0/0        0/168        0/2    0/0     0/1      ❓  │   │   │   ├── concurrent-queue 2.2.0
0/4        0/94         0/16   0/0     0/3      ❓  │   │   │   │   └── crossbeam-utils 0.8.16
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │       └── cfg-if 1.0.0
0/0        0/116        0/8    0/0     0/0      ❓  │   │   │   ├── event-listener 2.5.3
0/0        0/37         0/2    0/0     0/0      ❓  │   │   │   └── futures-core 0.3.28
0/4        0/230        0/40   0/0     0/12     ❓  │   │   ├── async-lock 2.8.0
0/1        0/858        0/4    0/0     0/12     ❓  │   │   ├── async-task 4.4.0
0/0        0/33         0/2    0/0     0/0      ❓  │   │   ├── atomic-waker 1.1.1
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── fastrand 1.9.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 1.13.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── fastrand 1.9.0
0/0        0/37         0/2    0/0     0/0      ❓  │   │   │   ├── futures-core 0.3.28
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── futures-io 0.3.28
0/41       0/2501       0/2    0/0     0/147    ❓  │   │   │   ├── memchr 2.6.3
0/2        0/20         0/1    0/0     0/0      ❓  │   │   │   │   └── log 0.4.20
0/0        0/5          0/0    0/0     0/0      ❓  │   │   │   │       └── serde 1.0.188
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │           └── serde_derive 1.0.188
0/0        0/15         0/0    0/0     0/3      ❓  │   │   │   │               ├── proc-macro2 1.0.67
0/0        0/4          0/0    0/0     0/0      ❓  │   │   │   │               │   └── unicode-ident 1.0.12
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │               ├── quote 1.0.33
0/0        0/15         0/0    0/0     0/3      ❓  │   │   │   │               │   └── proc-macro2 1.0.67
0/0        0/79         0/3    0/0     0/2      ❓  │   │   │   │               └── syn 2.0.37
0/0        0/15         0/0    0/0     0/3      ❓  │   │   │   │                   ├── proc-macro2 1.0.67
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │                   ├── quote 1.0.33
0/0        0/4          0/0    0/0     0/0      ❓  │   │   │   │                   └── unicode-ident 1.0.12
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── parking 2.1.0
0/0        0/191        0/0    0/0     0/2      ❓  │   │   │   ├── pin-project-lite 0.2.13
0/0        0/21         0/0    0/0     0/4      ❓  │   │   │   └── waker-fn 1.1.0
0/2        0/20         0/1    0/0     0/0      ❓  │   │   └── log 0.4.20
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 1.13.0
                                                       │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   └── autocfg 1.1.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── async-net 1.7.0
                                                       │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   └── autocfg 1.1.0
0/0        0/4          0/0    0/0     0/0      ❓  │   ├── async-io 1.13.0
0/4        0/230        0/40   0/0     0/12     ❓  │   │   ├── async-lock 2.8.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── cfg-if 1.0.0
0/0        0/168        0/2    0/0     0/1      ❓  │   │   ├── concurrent-queue 2.2.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 1.13.0
0/2        0/20         0/1    0/0     0/0      ❓  │   │   ├── log 0.4.20
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── parking 2.1.0
0/1        0/250        0/16   0/4     0/5      ❓  │   │   ├── polling 2.8.0
                                                       │   │   │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   └── autocfg 1.1.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── cfg-if 1.0.0
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   ├── libc 0.2.148
0/2        0/20         0/1    0/0     0/0      ❓  │   │   │   └── log 0.4.20
0/371      0/6690       0/2    0/0     0/22     ❓  │   │   ├── rustix 0.37.23
                                                       │   │   │   [build-dependencies]
0/1        0/232        0/2    0/0     0/4      ❓  │   │   │   └── cc 1.0.83
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │       └── libc 0.2.148
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── bitflags 1.3.2
0/0        0/100        0/0    0/0     0/0      ❓  │   │   │   ├── errno 0.3.3
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   │   └── libc 0.2.148
0/0        0/666        0/36   0/2     0/14     ❓  │   │   │   ├── io-lifetimes 1.0.11
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   │   ├── libc 0.2.148
0/6        0/673        0/4    0/0     0/4      ❓  │   │   │   │   └── socket2 0.4.9
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   │       └── libc 0.2.148
0/0        0/7          0/0    0/0     0/0      ❓  │   │   │   ├── itoa 1.0.9
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   └── libc 0.2.148
0/0        0/24         0/0    0/0     0/3      ❓  │   │   ├── slab 0.4.9
                                                       │   │   │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   └── autocfg 1.1.0
0/0        0/5          0/0    0/0     0/0      ❓  │   │   │   └── serde 1.0.188
0/6        0/673        0/4    0/0     0/4      ❓  │   │   ├── socket2 0.4.9
0/0        0/21         0/0    0/0     0/4      ❓  │   │   └── waker-fn 1.1.0
                                                       │   │   [build-dependencies]
0/0        0/0          0/0    0/0     0/0      ❓  │   │   └── autocfg 1.1.0
0/0        0/28         0/4    0/0     0/0      ❓  │   ├── blocking 1.3.1
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 1.13.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── fixed-buffer 0.5.0
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-io 0.3.28
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-io 0.3.28
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-lite 1.13.0
0/0        0/0          0/0    0/0     0/0      ❓  ├── include_dir 0.7.3
0/0        0/0          0/0    0/0     0/0      ❓  │   └── include_dir_macros 0.7.3
0/0        0/15         0/0    0/0     0/3      ❓  │       ├── proc-macro2 1.0.67
0/0        0/0          0/0    0/0     0/0      ❓  │       └── quote 1.0.33
0/0        0/121        0/9    0/0     0/4      ❓  ├── once_cell 1.18.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── permit 0.2.1
0/0        0/32         0/0    0/0     0/0      ❓  ├── rand 0.8.5
0/60       0/502        0/2    0/0     0/50     ❓  │   ├── libc 0.2.148
0/2        0/20         0/1    0/0     0/0      ❓  │   ├── log 0.4.20
0/0        0/0          0/0    0/0     0/0      ❓  │   ├── rand_chacha 0.3.1
0/2        0/712        0/0    0/0     0/25     ❓  │   │   ├── ppv-lite86 0.2.17
0/0        0/2          0/0    0/0     0/0      ❓  │   │   ├── rand_core 0.6.4
0/7        0/228        0/1    0/0     0/3      ❓  │   │   │   ├── getrandom 0.2.10
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │   ├── cfg-if 1.0.0
0/60       0/502        0/2    0/0     0/50     ❓  │   │   │   │   └── libc 0.2.148
0/0        0/5          0/0    0/0     0/0      ❓  │   │   │   └── serde 1.0.188
0/0        0/5          0/0    0/0     0/0      ❓  │   │   └── serde 1.0.188
0/0        0/2          0/0    0/0     0/0      ❓  │   ├── rand_core 0.6.4
0/0        0/5          0/0    0/0     0/0      ❓  │   └── serde 1.0.188
0/0        0/0          0/0    0/0     0/0      🔒  ├── safe-regex 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safe-regex-macro 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │       ├── safe-proc-macro2 1.0.67
0/0        0/4          0/0    0/0     0/0      ❓  │       │   └── unicode-ident 1.0.12
0/0        0/0          0/0    0/0     0/0      🔒  │       └── safe-regex-compiler 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │           ├── safe-proc-macro2 1.0.67
0/0        0/0          0/0    0/0     0/0      🔒  │           └── safe-quote 1.0.15
0/0        0/0          0/0    0/0     0/0      🔒  │               └── safe-proc-macro2 1.0.67
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-executor 0.3.3
0/0        0/0          0/0    0/0     0/0      🔒  │   ├── safina-sync 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safina-threadpool 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-sync 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-timer 0.1.11
0/0        0/121        0/9    0/0     0/4      ❓  │   └── once_cell 1.18.0
0/0        0/5          0/0    0/0     0/0      ❓  ├── serde 1.0.188
0/0        0/7          0/0    0/0     0/0      ❓  ├── serde_json 1.0.107
0/0        0/7          0/0    0/0     0/0      ❓  │   ├── itoa 1.0.9
0/9        0/715        0/0    0/0     0/2      ❓  │   ├── ryu 1.0.15
0/0        0/5          0/0    0/0     0/0      ❓  │   └── serde 1.0.188
0/0        0/0          0/0    0/0     0/0      🔒  ├── serde_urlencoded 0.7.1
0/0        0/2          0/0    0/0     0/0      ❓  │   ├── form_urlencoded 1.2.0
0/0        0/8          0/0    0/0     0/0      ❓  │   │   └── percent-encoding 2.3.0
0/0        0/7          0/0    0/0     0/0      ❓  │   ├── itoa 1.0.9
0/9        0/715        0/0    0/0     0/2      ❓  │   ├── ryu 1.0.15
0/0        0/5          0/0    0/0     0/0      ❓  │   └── serde 1.0.188
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-dir 0.1.11
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-file 0.1.7
0/0        0/0          0/0    0/0     0/0      ❓  └── url 2.4.1
0/0        0/2          0/0    0/0     0/0      ❓      ├── form_urlencoded 1.2.0
0/0        0/0          0/0    0/0     0/0      ❓      ├── idna 0.4.0
0/0        0/5          0/0    0/0     0/0      ❓      │   ├── unicode-bidi 0.3.13
0/0        0/5          0/0    0/0     0/0      ❓      │   │   └── serde 1.0.188
0/0        0/20         0/0    0/0     0/0      ❓      │   └── unicode-normalization 0.1.22
0/0        0/0          0/0    0/0     0/0      🔒      │       └── tinyvec 1.6.0
0/0        0/5          0/0    0/0     0/0      ❓      │           ├── serde 1.0.188
0/0        0/0          0/0    0/0     0/0      🔒      │           └── tinyvec_macros 0.1.1
0/0        0/8          0/0    0/0     0/0      ❓      ├── percent-encoding 2.3.0
0/0        0/5          0/0    0/0     0/0      ❓      └── serde 1.0.188

0/509      0/15404      0/156  0/6     0/324  

```
# Alternatives
See [rust-webserver-comparison.md](https://github.com/mleonhard/servlin/blob/main/rust-webserver-comparison.md).

# Changelog
- v0.4.2 - Implement `Seek` for `BodyReader`.
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
