use crate::test_util::check_elapsed;
use permit::Permit;
use safina::sync::Receiver;
use servlin::{socket_addr_127_0_0_1_any_port, HttpServerBuilder};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

mod test_util;

#[test]
fn server_quick_shutdown() {
    safina::timer::start_timer_thread();
    let permit = Permit::new();
    let executor = safina::executor::Executor::new(1, 1).unwrap();
    let (_, stopped_receiver): (SocketAddr, Receiver<()>) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .permit(permit.new_sub())
                .spawn(|_req| unreachable!()),
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let before = Instant::now();
    drop(permit);
    stopped_receiver
        .recv_timeout(Duration::from_millis(500))
        .unwrap();
    check_elapsed(before, 0..100).unwrap();
}
