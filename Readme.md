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
use std::sync::Arc;
use temp_dir::TempDir;

struct State {}

fn hello(_state: Arc<State>, req: Request) -> Result<Response, Error> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let input: Input = req.json()?;
    Ok(Response::json(200, json!({"message": format!("Hello, {}!", input.name)}))?)
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
safina::timer::start_timer_thread();
let executor = safina::executor::Executor::new(1, 9).unwrap();
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

0/0        0/0          0/0    0/0     0/0      🔒  servlin 0.7.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── safina 0.6.1
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safina-macros 0.1.3
0/0        0/0          0/0    0/0     0/0      🔒  │       ├── safe-proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      🔒  │       │   └── unicode-xid 0.2.6
0/0        0/0          0/0    0/0     0/0      🔒  │       └── safe-quote 1.0.40
0/0        0/0          0/0    0/0     0/0      🔒  │           └── safe-proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      🔒  ├── async-fs 2.1.2
4/4        222/222      40/40  0/0     13/13    ☢️  │   ├── async-lock 3.4.0
0/0        2/2          0/0    0/0     0/0      ☢️  │   │   ├── event-listener-strategy 0.5.4
0/0        39/49        8/12   0/0     2/2      ☢️  │   │   │   ├── event-listener 5.4.0
0/0        183/183      2/2    0/0     1/1      ☢️  │   │   │   │   ├── concurrent-queue 2.5.0
4/4        12/75        4/16   0/0     0/3      ☢️  │   │   │   │   │   └── crossbeam-utils 0.8.21
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   │   ├── parking 2.2.1
0/0        11/191       0/0    0/0     2/2      ☢️  │   │   │   │   └── pin-project-lite 0.2.16
0/0        11/191       0/0    0/0     2/2      ☢️  │   │   │   └── pin-project-lite 0.2.16
0/0        39/49        8/12   0/0     2/2      ☢️  │   │   ├── event-listener 5.4.0
0/0        11/191       0/0    0/0     2/2      ☢️  │   │   └── pin-project-lite 0.2.16
0/0        0/0          0/0    0/0     0/0      🔒  │   ├── blocking 1.6.1
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── async-channel 2.3.1
0/0        183/183      2/2    0/0     1/1      ☢️  │   │   │   ├── concurrent-queue 2.5.0
0/0        2/2          0/0    0/0     0/0      ☢️  │   │   │   ├── event-listener-strategy 0.5.4
0/0        36/36        2/2    0/0     0/0      ☢️  │   │   │   ├── futures-core 0.3.31
0/0        11/191       0/0    0/0     2/2      ☢️  │   │   │   └── pin-project-lite 0.2.16
1/1        860/866      4/4    0/0     13/13    ☢️  │   │   ├── async-task 4.7.1
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-io 0.3.31
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 2.6.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── fastrand 2.3.0
0/0        36/36        2/2    0/0     0/0      ☢️  │   │   │   ├── futures-core 0.3.31
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── futures-io 0.3.31
34/41      1700/2421    2/2    0/0     82/147   ☢️  │   │   │   ├── memchr 2.7.5
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── parking 2.2.1
0/0        11/191       0/0    0/0     2/2      ☢️  │   │   │   └── pin-project-lite 0.2.16
0/0        28/28        2/2    0/0     0/0      ☢️  │   │   ├── piper 0.2.4
0/0        32/32        2/2    0/0     0/0      ☢️  │   │   │   ├── atomic-waker 1.1.2
0/0        0/0          0/0    0/0     0/0      🔒  │   │   │   ├── fastrand 2.3.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   └── futures-io 0.3.31
0/0        14/14        1/1    0/0     0/0      ☢️  │   │   └── tracing 0.1.41
0/0        11/191       0/0    0/0     2/2      ☢️  │   │       ├── pin-project-lite 0.2.16
0/0        98/98        5/5    0/0     2/2      ☢️  │   │       └── tracing-core 0.1.34
0/0        0/124        0/9    0/0     0/5      ❓  │   │           └── once_cell 1.21.3
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 2.6.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── async-net 2.0.0
0/0        72/118       19/22  1/1     5/9      ☢️  │   ├── async-io 2.4.1
4/4        222/222      40/40  0/0     13/13    ☢️  │   │   ├── async-lock 3.4.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── cfg-if 1.0.1
0/0        183/183      2/2    0/0     1/1      ☢️  │   │   ├── concurrent-queue 2.5.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-io 0.3.31
0/0        0/0          0/0    0/0     0/0      ❓  │   │   ├── futures-lite 2.6.0
0/0        0/0          0/0    0/0     0/0      🔒  │   │   ├── parking 2.2.1
0/2        39/425       5/20   1/4     5/14     ☢️  │   │   ├── polling 3.8.0
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── cfg-if 1.0.1
61/433     2727/7465    18/22  2/2     41/62    ☢️  │   │   │   ├── rustix 1.0.7
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │   ├── bitflags 2.9.1
0/0        5/5          0/0    0/0     0/0      ☢️  │   │   │   │   │   └── serde 1.0.219
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │   │       └── serde_derive 1.0.219
0/0        14/14        0/0    0/0     3/3      ☢️  │   │   │   │   │           ├── proc-macro2 1.0.95
0/0        4/4          0/0    0/0     0/0      ☢️  │   │   │   │   │           │   └── unicode-ident 1.0.18
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │   │           ├── quote 1.0.40
0/0        14/14        0/0    0/0     3/3      ☢️  │   │   │   │   │           │   └── proc-macro2 1.0.95
0/0        88/88        3/3    0/0     2/2      ☢️  │   │   │   │   │           └── syn 2.0.104
0/0        14/14        0/0    0/0     3/3      ☢️  │   │   │   │   │               ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   │   │               ├── quote 1.0.40
0/0        4/4          0/0    0/0     0/0      ☢️  │   │   │   │   │               └── unicode-ident 1.0.18
0/0        35/103       0/0    0/0     0/0      ☢️  │   │   │   │   ├── errno 0.3.13
1/90       10/679       0/2    0/0     5/92     ☢️  │   │   │   │   │   └── libc 0.2.174
1/90       10/679       0/2    0/0     5/92     ☢️  │   │   │   │   └── libc 0.2.174
0/0        14/14        1/1    0/0     0/0      ☢️  │   │   │   └── tracing 0.1.41
61/433     2727/7465    18/22  2/2     41/62    ☢️  │   │   ├── rustix 1.0.7
0/0        29/29        0/0    0/0     3/3      ☢️  │   │   ├── slab 0.4.10
0/0        5/5          0/0    0/0     0/0      ☢️  │   │   │   └── serde 1.0.219
0/0        14/14        1/1    0/0     0/0      ☢️  │   │   └── tracing 0.1.41
0/0        0/0          0/0    0/0     0/0      🔒  │   ├── blocking 1.6.1
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-lite 2.6.0
0/0        0/0          0/0    0/0     0/0      🔒  ├── fixed-buffer 1.0.1
0/0        0/0          0/0    0/0     0/0      ❓  │   └── futures-io 0.3.31
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-io 0.3.31
0/0        0/0          0/0    0/0     0/0      ❓  ├── futures-lite 2.6.0
0/0        0/0          0/0    0/0     0/0      ❓  ├── include_dir 0.7.4
0/0        0/0          0/0    0/0     0/0      ❓  │   └── include_dir_macros 0.7.4
0/0        14/14        0/0    0/0     3/3      ☢️  │       ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓  │       └── quote 1.0.40
0/0        0/0          0/0    0/0     0/0      🔒  ├── permit 0.2.1
0/0        12/32        0/0    0/0     0/0      ☢️  ├── rand 0.8.5
1/90       10/679       0/2    0/0     5/92     ☢️  │   ├── libc 0.2.174
0/0        2/2          0/0    0/0     0/0      ☢️  │   ├── rand_core 0.6.4
3/6        51/192       0/1    0/0     1/3      ☢️  │   │   ├── getrandom 0.2.16
0/0        0/0          0/0    0/0     0/0      ❓  │   │   │   ├── cfg-if 1.0.1
1/90       10/679       0/2    0/0     5/92     ☢️  │   │   │   └── libc 0.2.174
0/0        5/5          0/0    0/0     0/0      ☢️  │   │   └── serde 1.0.219
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.219
0/0        0/0          0/0    0/0     0/0      🔒  ├── safe-regex 0.3.0
0/0        0/0          0/0    0/0     0/0      🔒  │   └── safe-regex-macro 0.3.0
0/0        0/0          0/0    0/0     0/0      🔒  │       ├── safe-proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      🔒  │       └── safe-regex-compiler 0.3.0
0/0        0/0          0/0    0/0     0/0      🔒  │           ├── safe-proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      🔒  │           └── safe-quote 1.0.40
0/0        5/5          0/0    0/0     0/0      ☢️  ├── serde 1.0.219
0/0        72/75        0/0    0/0     0/0      ☢️  ├── serde_json 1.0.140
0/0        8/8          0/0    0/0     0/0      ☢️  │   ├── itoa 1.0.15
34/41      1700/2421    2/2    0/0     82/147   ☢️  │   ├── memchr 2.7.5
7/9        572/702      0/0    0/0     2/2      ☢️  │   ├── ryu 1.0.20
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.219
0/0        0/0          0/0    0/0     0/0      🔒  ├── serde_urlencoded 0.7.1
0/0        2/2          0/0    0/0     0/0      ☢️  │   ├── form_urlencoded 1.2.1
0/0        8/8          0/0    0/0     0/0      ☢️  │   │   └── percent-encoding 2.3.1
0/0        8/8          0/0    0/0     0/0      ☢️  │   ├── itoa 1.0.15
7/9        572/702      0/0    0/0     2/2      ☢️  │   ├── ryu 1.0.20
0/0        5/5          0/0    0/0     0/0      ☢️  │   └── serde 1.0.219
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-dir 0.1.16
0/0        0/0          0/0    0/0     0/0      🔒  ├── temp-file 0.1.9
0/0        0/0          0/0    0/0     0/0      ❓  └── url 2.5.4
0/0        2/2          0/0    0/0     0/0      ☢️      ├── form_urlencoded 1.2.1
0/0        30/30        0/0    0/0     0/0      ☢️      ├── idna 1.0.3
0/0        0/0          0/0    0/0     0/0      ❓      │   ├── idna_adapter 1.2.1
0/0        23/23        0/0    0/0     0/0      ☢️      │   │   ├── icu_normalizer 2.0.0
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   ├── displaydoc 0.2.5
0/0        14/14        0/0    0/0     3/3      ☢️      │   │   │   │   ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   ├── quote 1.0.40
0/0        88/88        3/3    0/0     2/2      ☢️      │   │   │   │   └── syn 2.0.104
0/0        1/1          0/0    0/0     1/1      ☢️      │   │   │   ├── icu_collections 2.0.0
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   ├── displaydoc 0.2.5
0/0        6/24         2/2    0/0     2/2      ☢️      │   │   │   │   ├── potential_utf 0.1.2
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── serde 1.0.219
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── writeable 0.6.1
0/0        0/2          0/0    0/0     0/0      ❓      │   │   │   │   │   │   └── either 1.15.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   │       └── serde 1.0.219
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   │   └── zerovec 0.11.2
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │       ├── serde 1.0.219
0/0        96/101       24/25  4/4     12/13    ☢️      │   │   │   │   │       ├── yoke 0.8.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │       │   ├── serde 1.0.219
0/0        0/0          18/18  2/2     0/0      ☢️      │   │   │   │   │       │   ├── stable_deref_trait 1.2.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │   ├── yoke-derive 0.8.0
0/0        14/14        0/0    0/0     3/3      ☢️      │   │   │   │   │       │   │   ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │   │   ├── quote 1.0.40
0/0        88/88        3/3    0/0     2/2      ☢️      │   │   │   │   │       │   │   ├── syn 2.0.104
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │   │   └── synstructure 0.13.2
0/0        14/14        0/0    0/0     3/3      ☢️      │   │   │   │   │       │   │       ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │   │       ├── quote 1.0.40
0/0        88/88        3/3    0/0     2/2      ☢️      │   │   │   │   │       │   │       └── syn 2.0.104
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │   └── zerofrom 0.1.6
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │       └── zerofrom-derive 0.1.6
0/0        14/14        0/0    0/0     3/3      ☢️      │   │   │   │   │       │           ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │           ├── quote 1.0.40
0/0        88/88        3/3    0/0     2/2      ☢️      │   │   │   │   │       │           ├── syn 2.0.104
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       │           └── synstructure 0.13.2
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │       ├── zerofrom 0.1.6
0/0        0/0          0/1    0/0     0/0      ❓      │   │   │   │   │       └── zerovec-derive 0.11.1
0/0        14/14        0/0    0/0     3/3      ☢️      │   │   │   │   │           ├── proc-macro2 1.0.95
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │           ├── quote 1.0.40
0/0        88/88        3/3    0/0     2/2      ☢️      │   │   │   │   │           └── syn 2.0.104
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   ├── serde 1.0.219
0/0        96/101       24/25  4/4     12/13    ☢️      │   │   │   │   ├── yoke 0.8.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   ├── zerofrom 0.1.6
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   └── zerovec 0.11.2
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   ├── icu_normalizer_data 2.0.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   ├── icu_properties 2.0.1
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   ├── displaydoc 0.2.5
0/0        1/1          0/0    0/0     1/1      ☢️      │   │   │   │   ├── icu_collections 2.0.0
0/0        11/11        0/0    0/0     0/0      ☢️      │   │   │   │   ├── icu_locale_core 2.0.0
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   │   ├── displaydoc 0.2.5
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │   ├── litemap 0.8.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   │   ├── serde 1.0.219
0/0        96/101       24/25  4/4     12/13    ☢️      │   │   │   │   │   │   └── yoke 0.8.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── serde 1.0.219
0/0        36/37        2/2    0/0     2/2      ☢️      │   │   │   │   │   ├── tinystr 0.8.1
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   │   │   ├── displaydoc 0.2.5
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   │   ├── serde 1.0.219
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   │   │   └── zerovec 0.11.2
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── writeable 0.6.1
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   │   └── zerovec 0.11.2
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   ├── icu_properties_data 2.0.1
0/0        31/31        3/3    0/0     2/2      ☢️      │   │   │   │   ├── icu_provider 2.0.0
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   │   ├── displaydoc 0.2.5
0/0        11/11        0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── icu_locale_core 2.0.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── serde 1.0.219
0/0        72/75        0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── serde_json 1.0.140
0/0        0/0          18/18  2/2     0/0      ☢️      │   │   │   │   │   ├── stable_deref_trait 1.2.0
0/0        36/37        2/2    0/0     2/2      ☢️      │   │   │   │   │   ├── tinystr 0.8.1
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── writeable 0.6.1
0/0        96/101       24/25  4/4     12/13    ☢️      │   │   │   │   │   ├── yoke 0.8.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │   ├── zerofrom 0.1.6
0/0        9/12         0/0    0/0     0/0      ☢️      │   │   │   │   │   ├── zerotrie 0.2.2
0/12       0/12         0/0    0/0     0/0      ❓      │   │   │   │   │   │   ├── displaydoc 0.2.5
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │   │   ├── litemap 0.8.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   │   │   ├── serde 1.0.219
0/0        96/101       24/25  4/4     12/13    ☢️      │   │   │   │   │   │   ├── yoke 0.8.0
0/0        0/0          0/0    0/0     0/0      ❓      │   │   │   │   │   │   ├── zerofrom 0.1.6
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   │   │   └── zerovec 0.11.2
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   │   └── zerovec 0.11.2
0/0        6/24         2/2    0/0     2/2      ☢️      │   │   │   │   ├── potential_utf 0.1.2
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   ├── serde 1.0.219
0/0        9/12         0/0    0/0     0/0      ☢️      │   │   │   │   ├── zerotrie 0.2.2
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   │   └── zerovec 0.11.2
0/0        31/31        3/3    0/0     2/2      ☢️      │   │   │   ├── icu_provider 2.0.0
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   ├── serde 1.0.219
1/1        554/556      7/7    1/1     14/14    ☢️      │   │   │   ├── smallvec 1.15.1
0/0        5/5          0/0    0/0     0/0      ☢️      │   │   │   │   └── serde 1.0.219
0/0        10/10        0/0    0/0     0/0      ☢️      │   │   │   ├── utf8_iter 1.0.4
1/1        641/657      58/58  5/5     49/49    ☢️      │   │   │   └── zerovec 0.11.2
0/0        0/0          0/0    0/0     0/0      ❓      │   │   └── icu_properties 2.0.1
1/1        554/556      7/7    1/1     14/14    ☢️      │   ├── smallvec 1.15.1
0/0        10/10        0/0    0/0     0/0      ☢️      │   └── utf8_iter 1.0.4
0/0        8/8          0/0    0/0     0/0      ☢️      ├── percent-encoding 2.3.1
0/0        5/5          0/0    0/0     0/0      ☢️      └── serde 1.0.219

117/604    8440/15804   231/283 16/19   264/461

```
# Alternatives
See [rust-webserver-comparison.md](https://github.com/mleonhard/servlin/blob/main/rust-webserver-comparison.md).

# Changelog
- v0.7.0 2025-01-03
  - `log_request_and_response` to log `duration_ms` tag.
  - Fix typo in function name `Response::internal_server_errror_500`.
  - Close connection on 5xx error.
  - Acceptor thread to log errors, not panic.
  - Add [`Request::parse_url`].
  - Add [`Response::too_many_requests_429`].
  - Implement `Into<TagList>` for arrays.
  - Support asterisk request target.
- v0.6.0 2024-11-02
  - Remove `servlin::reexports` module.
  - Use `safina` v0.6.0.
- v0.5.1 2024-10-26 - Remove dependency on `once_cell`.
- v0.5.0 2024-10-21 - Remove `LogFileWriterBuilder`.
- v0.4.3 - Implement `From<Cow<'_, str>>` and `From<&Path>` for `TagValue`.
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
