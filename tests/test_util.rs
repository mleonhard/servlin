#![cfg(test)]
#![cfg(feature = "internals")]
use beatrice::internals::listen_127_0_0_1_any_port;
use std::future::Future;
use std::time::Duration;

#[allow(clippy::missing_panics_doc)]
pub fn async_test<Fut: Future<Output = ()> + Send + 'static>(fut: Fut) {
    safina_timer::start_timer_thread();
    safina_executor::Executor::new(2, 1)
        .unwrap()
        .block_on(safina_timer::with_timeout(fut, Duration::from_secs(10)))
        .unwrap();
}

#[allow(clippy::missing_panics_doc)]
pub async fn connected_streams() -> (async_net::TcpStream, async_net::TcpStream) {
    let listener = listen_127_0_0_1_any_port().await.unwrap();
    let listen_addr = listener.local_addr().unwrap();
    let (sender, mut receiver) = safina_sync::oneshot();
    safina_executor::spawn(async move {
        let _result = sender.send(listener.accept().await.unwrap().0);
    });
    let stream0 = async_net::TcpStream::connect(listen_addr).await.unwrap();
    let stream1 = receiver.async_recv().await.unwrap();
    (stream0, stream1)
}
