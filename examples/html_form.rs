//! HTML Form Example
//! =================
//!
//! Start the server:
//! ```
//! cargo run --package servlin --features urlencoded --example html_form
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.25s
//!      Running `target/debug/examples/html_form`
//! Access the server at http://127.0.0.1:8000/
//! 2023-04-20T17:18:57Z info "code":200,"body_len":370,"http_method":"GET","path":"/","request_id":3504837461057921534
//! 2023-04-20T17:18:57Z info "code":404,"body_len":9,"http_method":"GET","path":"/favicon.ico","request_id":14979834421061568265
//! 2023-04-20T17:19:08Z info "code":303,"body_len":0,"http_method":"POST","path":"/increment","request_id":10633635939599229141
//! 2023-04-20T17:19:08Z info "code":200,"body_len":370,"http_method":"GET","path":"/","request_id":2615853109542701666
//! 2023-04-20T17:19:08Z info "code":404,"body_len":9,"http_method":"GET","path":"/favicon.ico","request_id":6599307416411604969
//! 2023-04-20T17:19:15Z info "code":303,"body_len":0,"http_method":"POST","path":"/increment","request_id":1471059950980094153
//! 2023-04-20T17:19:15Z info "code":200,"body_len":370,"http_method":"GET","path":"/","request_id":13807176225544983707
//! 2023-04-20T17:19:15Z info "code":404,"body_len":9,"http_method":"GET","path":"/favicon.ico","request_id":6733001312936409561
//! 2023-04-20T17:19:24Z info "code":303,"body_len":0,"http_method":"POST","path":"/add","request_id":2450901135286845771
//! 2023-04-20T17:19:24Z info "code":200,"body_len":370,"http_method":"GET","path":"/","request_id":18076069727349126411
//! ^C
//! ```
//!
//! Access the form with your web browser:
//! <http://127.0.0.1:8000/>
#![forbid(unsafe_code)]
use safina::executor::Executor;
use serde::Deserialize;
use servlin::log::log_request_and_response;
use servlin::{Error, HttpServerBuilder, Request, Response, socket_addr_127_0_0_1};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct State {
    count: AtomicUsize,
}
impl State {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
    }

    pub fn add(&self, n: usize) {
        self.count.fetch_add(n, Ordering::AcqRel);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn index(state: Arc<State>) -> Response {
    Response::html(
        200,
        format!(
            "<html>
<head><title>Example</title></head>
<body>
  <h1>Example</h1>
  <p>Count: {}</p>
  <form action='/increment' method='post'>
    <input type='submit' value='Increment'/><br/>
  </form>
  <form action='/add' method='post'>
    <label>Num <input type='number' autofocus name='num' /></label>
    <input type='submit' name='add' value='Add'/>
  </form>
</body>
</html>",
            state.get()
        ),
    )
}

#[allow(clippy::needless_pass_by_value)]
fn increment(state: Arc<State>) -> Response {
    state.increment();
    Response::redirect_303("/")
}

#[allow(clippy::needless_pass_by_value)]
fn add(state: Arc<State>, req: Request) -> Result<Response, Error> {
    #[derive(Deserialize)]
    struct Input {
        num: usize,
    }
    let input: Input = req.urlencoded()?;
    let num = if input.num > 5 {
        return Err(Error::client_error(Response::text(400, "num is too big")));
    } else {
        input.num
    };
    state.add(num);
    Ok(Response::redirect_303("/"))
}

fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
    match (req.method(), req.url().path()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("GET", "/") => Ok(index(state)),
        ("POST", "/increment") => Ok(increment(state)),
        ("POST", "/add") => add(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    println!("Access the server at http://127.0.0.1:8000/");
    safina::timer::start_timer_thread();
    let executor: Arc<Executor> = Arc::default();
    let state = Arc::new(State::new());
    let request_handler =
        move |req: Request| log_request_and_response(req, |req| handle_req(state, req)).unwrap();
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .max_conns(100)
                .spawn_and_join(request_handler),
        )
        .unwrap();
}
