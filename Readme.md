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
    print_log_response,
    socket_addr_127_0_0_1,
    HttpServerBuilder,
    Request,
    Response
};
use servlin::reexport::{safina_executor, safina_timer};
use std::sync::Arc;
use temp_dir::TempDir;

struct State {}

fn hello(_state: Arc<State>, req: &Request) -> Result<Response, Response> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let input: Input = req.json()?;
    Ok(Response::json(200, json!({"message": format!("Hello, {}!", input.name)}))
    .unwrap())
}

fn handle_req(state: Arc<State>, req: &Request) -> Result<Response, Response> {
    match (req.method(), req.url().path()) {
        ("GET", "/ping") => Ok(Response::text(200, "ok")),
        ("POST", "/hello") => hello(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

let state = Arc::new(State {});
let request_handler = move |req: Request| {
    print_log_response(&req, handle_req(state, &req))
};
let cache_dir = TempDir::new().unwrap();
safina_timer::start_timer_thread();
let executor = safina_executor::Executor::new(1, 9).unwrap();
executor.block_on(
    HttpServerBuilder::new()
        .listen_addr(socket_addr_127_0_0_1(8009))
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

0/0        0/0          0/0    0/0     0/0      🔒  servlin 0.1.1
0/0        4/4          0/0    0/0     2/2      ☢️  ├── async-fs 1.5.0
0/0        51/51        14/14  0/0     0/0      ☢️  │   ├── async-lock 2.5.0
0/0        106/116      4/8    0/0     0/0      ☢️  │   │   └── event-listener 2.5.2
0/0        28/28        4/4    0/0     0/0      ☢️  │   ├── blocking 1.2.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── async-channel 1.6.1
0/0        155/155      2/2    0/0     1/1      ☢️  │   │   │   ├── concurrent-queue 1.2.2
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   │   └── cache-padded 1.2.0
0/0        106/116      4/8    0/0     0/0      ☢️  │   │   │   ├── event-listener 2.5.2
0/0        30/30        2/2    0/0     0/0      ☢️  │   │   │   └── futures-core 0.3.21
1/1        802/802      4/4    0/0     10/10    ☢️  │   │   ├── async-task 4.2.0
0/0        26/26        2/2    0/0     0/0      ☢️  │   │   ├── atomic-waker 1.0.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── fastrand 1.7.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 1.12.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── fastrand 1.7.0
0/0        30/30        2/2    0/0     0/0      ☢️  │   │   │   ├── futures-core 0.3.21
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── futures-io 0.3.21
36/37      2067/2144    0/0    0/0     21/21    ☢️  │   │   │   ├── memchr 2.5.0
1/21       10/368       0/2    0/0     5/40     ☢️  │   │   │   │   └── libc 0.2.126
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── parking 2.0.0
0/0        11/165       0/0    0/0     2/2      ☢️  │   │   │   ├── pin-project-lite 0.2.9
0/0        21/21        0/0    0/0     4/4      ☢️  │   │   │   └── waker-fn 1.1.0
1/1        75/117       4/6    0/0     2/3      ☢️  │   │   └── once_cell 1.12.0
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 1.12.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── async-net 1.6.1
0/0        22/22        0/0    0/0     0/0      ☢️  │   ├── async-io 1.7.0
0/0        155/155      2/2    0/0     1/1      ☢️  │   │   ├── concurrent-queue 1.2.2
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 1.12.0
1/21       10/368       0/2    0/0     5/40     ☢️  │   │   ├── libc 0.2.126
1/1        16/18        1/1    0/0     0/0      ☢️  │   │   ├── log 0.4.17
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── cfg-if 1.0.0
0/0        5/5          0/0    0/0     0/0      ☢️  │   │   │   └── serde 1.0.137
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │       └── serde_derive 1.0.137
0/0        12/12        0/0    0/0     3/3      ☢️  │   │   │           ├── proc-macro2 1.0.40
0/0        4/4          0/0    0/0     0/0      ☢️  │   │   │           │   └── unicode-ident 1.0.1
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │           ├── quote 1.0.20
0/0        12/12        0/0    0/0     3/3      ☢️  │   │   │           │   └── proc-macro2 1.0.40
0/0        50/50        3/3    0/0     2/2      ☢️  │   │   │           └── syn 1.0.98
0/0        12/12        0/0    0/0     3/3      ☢️  │   │   │               ├── proc-macro2 1.0.40
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │               ├── quote 1.0.20
0/0        4/4          0/0    0/0     0/0      ☢️  │   │   │               └── unicode-ident 1.0.1
1/1        75/117       4/6    0/0     2/3      ☢️  │   │   ├── once_cell 1.12.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── parking 2.0.0
0/0        0/9          1/6    0/0     0/0      ☢️  │   │   ├── polling 2.2.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── cfg-if 1.0.0
1/21       10/368       0/2    0/0     5/40     ☢️  │   │   │   ├── libc 0.2.126
1/1        16/18        1/1    0/0     0/0      ☢️  │   │   │   └── log 0.4.17
0/0        24/24        0/0    0/0     3/3      ☢️  │   │   ├── slab 0.4.6
0/0        5/5          0/0    0/0     0/0      ☢️  │   │   │   └── serde 1.0.137
3/6        528/641      2/4    0/0     3/4      ☢️  │   │   ├── socket2 0.4.4
1/21       10/368       0/2    0/0     5/40     ☢️  │   │   │   └── libc 0.2.126
0/0        21/21        0/0    0/0     4/4      ☢️  │   │   └── waker-fn 1.1.0
0/0        28/28        4/4    0/0     0/0      ☢️  │   ├── blocking 1.2.0
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 1.12.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── fixed-buffer 0.5.0
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-io 0.3.21
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-io 0.3.21
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-lite 1.12.0
0/0        0/0          0/0    0/0     0/0      ❓  ├── include_dir 0.7.2
0/0        0/0          0/0    0/0     0/0      ❓  │   └── include_dir_macros 0.7.2
0/0        12/12        0/0    0/0     3/3      ☢️  │       ├── proc-macro2 1.0.40
0/0        0/0          0/0    0/0     0/0      ❓  │       └── quote 1.0.20
0/0        0/0          0/0    0/0     0/0      🔒  ├── permit 0.1.4
0/0        0/0          0/0    0/0     0/0      🔒  ├── safe-regex 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safe-regex-macro 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │       ├── safe-proc-macro2 1.0.36
0/0        0/0          0/0    0/0     0/0      🔒  │       │   └── unicode-xid 0.2.3
0/0        0/0          0/0    0/0     0/0      🔒  │       └── safe-regex-compiler 0.2.5
0/0        0/0          0/0    0/0     0/0      🔒  │           ├── safe-proc-macro2 1.0.36
0/0        0/0          0/0    0/0     0/0      🔒  │           └── safe-quote 1.0.15
0/0        0/0          0/0    0/0     0/0      🔒  │               └── safe-proc-macro2 1.0.36
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-executor 0.3.3
0/0        0/0          0/0    0/0     0/0      🔒  │   ├── safina-sync 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safina-threadpool 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-sync 0.2.4
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina-timer 0.1.11
1/1        75/117       4/6    0/0     2/3      ☢️  │   └── once_cell 1.12.0
0/0        5/5          0/0    0/0     0/0      ☢️  ├── serde 1.0.137
0/0        4/7          0/0    0/0     0/0      ☢️  ├── serde_json 1.0.81
0/0        7/7          0/0    0/0     0/0      ☢️  │   ├── itoa 1.0.2
7/9        587/723      0/0    0/0     2/2      ☢️  │   ├── ryu 1.0.10
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.137
0/0        0/0          0/0    0/0     0/0      🔒  ├── serde_urlencoded 0.7.1
0/0        2/2          0/0    0/0     0/0      ☢️  │   ├── form_urlencoded 1.0.1
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── matches 0.1.9
0/0        3/3          0/0    0/0     0/0      ☢️  │   │   └── percent-encoding 2.1.0
0/0        7/7          0/0    0/0     0/0      ☢️  │   ├── itoa 1.0.2
7/9        587/723      0/0    0/0     2/2      ☢️  │   ├── ryu 1.0.10
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.137
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-dir 0.1.11
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-file 0.1.7
0/0        0/0          0/0    0/0     0/0      ❓  └── url 2.2.2
0/0        2/2          0/0    0/0     0/0      ☢️      ├── form_urlencoded 1.0.1
0/0        0/0          0/0    0/0     0/0      ❓      ├── idna 0.2.3
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── matches 0.1.9
0/0        0/0          0/0    0/0     0/0      🔒      │   ├── unicode-bidi 0.3.8
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   └── serde 1.0.137
0/0        20/20        0/0    0/0     0/0      ☢️      │   └── unicode-normalization 0.1.20
0/0        0/0          0/0    0/0     0/0      🔒      │       └── tinyvec 1.6.0
0/0        5/5          0/0    0/0     0/0      ☢️      │           ├── serde 1.0.137
0/0        0/0          0/0    0/0     0/0      ❓      │           └── tinyvec_macros 0.1.0
0/0        0/0          0/0    0/0     0/0      ❓      ├── matches 0.1.9
0/0        3/3          0/0    0/0     0/0      ☢️      ├── percent-encoding 2.1.0
0/0        5/5          0/0    0/0     0/0      ☢️      └── serde 1.0.137

50/76      4670/5574    43/58  0/0     60/97  

```
# Alternatives
See [rust-webserver-comparison.md](https://github.com/mleonhard/servlin/blob/main/rust-webserver-comparison.md).

# Changelog
- v0.1.1 - Add `EventSender::unconnected`.
- v0.1.0 - Rename library to Servlin.

# TO DO
- Fix limitations above
- Support [HEAD](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/HEAD)
  responses that have Content-Length set and no body.
- Update `rust-webserver-comparison.md`
  - Add missing data
  - Add other servers from <https://www.arewewebyet.org/topics/frameworks/>
  - Rearrange
  - Generate geiger reports for each web server

License: MIT OR Apache-2.0
