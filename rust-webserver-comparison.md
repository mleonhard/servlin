# Rust Webserver Comparison

|  | beatrice | [rouille](https://crates.io/crates/rouille) | [trillium](https://crates.io/crates/trillium) | [tide](https://crates.io/crates/tide) | [axum](https://crates.io/crates/axum) | [poem](https://crates.io/crates/poem) |
|---------------------|----|----|----|----|----|----|
| Well-tested         | [NO](https://github.com/mleonhard/beatrice-rs/issues/1) | NO | [NO](https://github.com/trillium-rs/trillium/discussions/181) | NO | NO | NO |
| Blocking handlers   | ✓  | ✓  | NO | NO | NO | ✓  |
| Async handlers      | NO | NO | ✓  | ✓  | ✓  | ✓  |
| 100-continue        | ✓  | ✓  | ✓  | [NO](https://github.com/http-rs/tide/issues/878) | ✓ | ✓ |
| Thread limit        | ✓  | [NO](https://github.com/tiny-http/tiny-http/issues/221) | ✓ | ✓ | ✓ | ✓ |
| Connection limit    | ✓  | NO | ✓  | NO | NO | NO |
| Caches payloads     | ✓  | NO | NO | NO | NO | [NO](https://github.com/poem-web/poem/issues/75) |
| Request timeouts    | NO | NO | NO | NO | NO | NO |
| Custom logging      | ✓  | ✓  | ✓  | NO | ✓  | ✓  |
| Unsafe-free         | ✓  | ✓  | NO | NO | NO | ✓  |
| Unsafe-free deps    | NO | NO | NO | NO | NO | NO |
| age (years)         | 0  | 6  | 1  | 3  | 0  | 1  |
| TLS                 | NO | NO | ✓  | ✓  | ✓  | ✓  |
| ACME certs          | NO | NO | NO | NO | NO | [NO](https://docs.rs/poem/1.3.29/poem/listener/acme/index.html) |
| SSE                 | ✓  | NO | [NO](https://github.com/trillium-rs/trillium/issues/39) | ✓ | ✓ | ✓ |
| Websockets          | NO | ✓  | ✓  | ✓  | ✓  | ✓  |
| Streaming response: |    |    |    |    |    |    |
| - impl `AsyncRead`  | NO | NO | ✓  | ✓  | ✓  | ✓  |
| - `AsyncWrite`      | NO | NO | NO | NO | NO | NO |
| - impl `Read`       | NO | ✓  | NO | NO | NO | NO |
| - channel           | NO | NO | NO | NO | ✓  | NO |
| Custom routing      | ✓  | ✓  | ✓  | NO | ✓  | ✓  |
| Usable sans macros  | ✓  | ✓  | ✓  | ✓  | ✓  | NO |
| Shutdown for tests  | ✓  | ✓  | ✓  | [NO](https://github.com/http-rs/tide/issues/876) | ✓ | ✓ |
| Graceful shutdown   | NO | ✓  | ✓  | [NO](https://github.com/http-rs/tide/issues/528) | ✓ | ✓ |
| Rust stable         | ✓  | ✓  | ✓  | ✓  | ✓  | ✓  |

|  | beatrice | [warp](https://crates.io/crates/warp) | [thruster](https://crates.io/crates/thruster) | [rocket](https://crates.io/crates/rocket) | [gotham](https://crates.io/crates/gotham) |
|---------------------|----|----|----|----|----|
| Well-tested         | [NO](https://github.com/mleonhard/beatrice-rs/issues/1) | ? | ? | ? | ? |
| Blocking handlers   | ✓  | ?  | ?  | ?  | ?  |
| Async handlers      | NO | ?  | ?  | ?  | ?  |
| 100-continue        | ✓  | ?  | ?  | ?  | ?  |
| Thread limit        | ✓  | ?  | ?  | ?  | ?  |
| Connection limit    | ✓  | ?  | ?  | ?  | ?  |
| Caches payloads     | ✓  | ?  | ?  | ?  | ?  |
| Request timeouts    | NO | ?  | ?  | ?  | ?  |
| Custom logging      | ✓  | ?  | ?  | ?  | ?  |
| Unsafe-free         | ✓  | ?  | ?  | ?  | ?  |
| Unsafe-free deps    | NO | ?  | ?  | ?  | ?  |
| age (years)         | 0  | ?  | ?  | ?  | 5  |
| TLS                 | NO | ?  | ?  | ?  | ?  |
| ACME certs          | NO | ?  | ?  | ?  | ?  |
| SSE                 | ✓  | ?  | ?  | ?  | ?  |
| Websockets          | NO | ?  | ?  | ?  | ?  |
| Streaming response: |    |    |    |    |    |
| - impl `AsyncRead`  | NO | ?  | ?  | ?  | ?  |
| - `AsyncWrite`      | NO | ?  | ?  | ?  | ?  |
| - impl `Read`       | NO | ?  | ?  | ?  | ?  |
| - channel           | NO | ?  | ?  | ?  | ?  |
| Custom routing      | ✓  | ?  | ?  | ?  | ?  |
| Usable sans macros  | ✓  | ?  | ?  | ?  | ?  |
| Shutdown for tests  | ✓  | ?  | ?  | ?  | ?  |
| Graceful shutdown   | NO | ?  | ?  | ?  | ?  |
| Rust stable         | ✓  | ?  | ?  | NO | ?  |
