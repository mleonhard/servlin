use beatrice::{print_log_response, run_http_server, socket_addr_127_0_0_1, Request, Response};
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct State {
    upload_count: AtomicUsize,
}

fn upload(state: &Arc<State>, req: Request) -> Result<Response, Response> {
    if req.body().is_pending() {
        println!("continue");
        return Ok(Response::GetBodyAndReprocess(1024 * 1024, req));
    }
    println!("upload receiving");
    let mut body_string = String::new();
    req.body().reader()?.read_to_string(&mut body_string)?;
    dbg!(&body_string);
    state.upload_count.fetch_add(1, Ordering::AcqRel);
    Ok(Response::text(
        200,
        format!(
            "Upload received, body_len={}, upload_count={}\n",
            body_string.len(),
            state.upload_count.load(Ordering::Acquire)
        ),
    ))
}

fn hello(req: &Request) -> Result<Response, Response> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let input: Input = req.json()?;
    Ok(Response::json(
        200,
        json!({ "message": format!("Helle, {}!  Nice to meet you.", input.name) }),
    )
    .unwrap())
}

fn handle_req(state: &Arc<State>, req: Request) -> Result<Response, Response> {
    match (req.method(), req.url().path(), req.content_type()) {
        ("GET", "/ping", _) => Ok(Response::text(200, "ok")),
        ("POST", "/hello", _) => hello(&req),
        ("POST", "/upload", _) => upload(state, req.recv_body(1024 * 1024)?),
        (_, "/upload", _) => Ok(Response::method_not_allowed_405(&["POST"])),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    let state = Arc::new(State {
        upload_count: AtomicUsize::new(0),
    });
    let request_handler = move |req: Request| {
        print_log_response(
            req.method().to_string(),
            req.url().clone(),
            handle_req(&state, req),
        )
    };
    run_http_server(
        socket_addr_127_0_0_1(8000),
        4,
        10,
        64 * 1024,
        request_handler,
    )
    .unwrap();
}
