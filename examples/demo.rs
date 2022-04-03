use beatrice::reexport::{safina_executor, safina_timer};
use beatrice::{print_log_response, socket_addr_127_0_0_1, HttpServerBuilder, Request, Response};
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use temp_dir::TempDir;

pub struct State {
    upload_count: AtomicUsize,
}

fn upload(state: &Arc<State>, req: &Request) -> Result<Response, Response> {
    if req.body.is_pending() {
        println!("continue");
        return Ok(Response::get_body_and_reprocess(1024 * 1024));
    }
    println!("upload receiving");
    let mut body_string = String::new();
    req.body.reader()?.read_to_string(&mut body_string)?;
    //dbg!(&body_string);
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
        ("POST", "/upload", _) => upload(state, &req.recv_body(1024 * 1024)?),
        (_, "/upload", _) => Ok(Response::method_not_allowed_405(&["POST"])),
        _ => Ok(Response::text(404, "Not found")),
    }
}

// async fn handle_req(
//     state: &Arc<State>,
//     http_conn: &mut HttpConn,
//     mut req: Request,
// ) -> Result<Response, HttpError> {
//     let blocking = |f| async {
//         with_timeout(schedule_blocking(f).async_recv(), Duration::from_secs(10)).await??
//     };
//     match (req.method(), req.url().path()) {
//         ("GET", "/ping") => Ok(Response::text(200, "ok")),
//         ("POST", "/hello") => blocking(|| hello(req)).await?,
//         ("GET", "/events") => blocking(|| events(req, state)).await?,
//         ("POST", "/upload") => {
//             if req.body.is_pending() {
//                 blocking(|| check_upload(state.clone(), req.clone())).await?;
//                 req.body = with_timeout(
//                     http_conn.read_body_to_file(cache_dir.path(), 1024 * 1024),
//                     Duration::from_secs(120),
//                 )
//                 .await??;
//             }
//             blocking(|| handle_upload(state, req)).await?
//         }
//         _ => Ok(Response::text(404, "Not found")),
//     }
// }

// let handler = move |http_conn: &mut HttpConn| {
//     let mut req = http_conn.read_request().await?;
//     print_logger(&req).log(handle_req(&state, http_conn, req).await)
// };

pub fn main() {
    safina_timer::start_timer_thread();
    let executor = safina_executor::Executor::default();
    let cache_dir = TempDir::new().unwrap();
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
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .max_conns(100)
                .small_body_len(64 * 1024)
                .receive_large_bodies(cache_dir.path())
                .spawn_and_join(request_handler),
        )
        .unwrap();
}
