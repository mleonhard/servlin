use crate::test_util::{read_for, TestServer};
use beatrice::{Event, Response};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod test_util;

#[test]
fn already_closed() {
    let server = TestServer::start(move |_req| {
        let (sender, response) = Response::event_stream();
        drop(sender);
        response
    })
    .unwrap();
    let mut tcp_stream = server.connect_and_send("M / HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n0\r\n\r\n",
    );
}

#[test]
fn empty() {
    let server = TestServer::start(move |_req| {
        let (sender, response) = Response::event_stream();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(sender);
        });
        response
    })
    .unwrap();
    let mut tcp_stream = server.connect_and_send("M / HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n",
    );
    assert_eq!(read_for(&mut tcp_stream, 200).unwrap(), "0\r\n\r\n");
}

#[test]
fn single_message() {
    let server = TestServer::start(move |_req| {
        let (mut sender, response) = Response::event_stream();
        std::thread::spawn(move || {
            sender.send(Event::Message("msg1".to_string()));
        });
        response
    })
    .unwrap();
    let mut tcp_stream = server.connect_and_send("M / HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\nb\r\ndata: msg1\n\r\n0\r\n\r\n",
    );
}

#[test]
fn multiple_messages() {
    let (test_sender, test_receiver) = std::sync::mpsc::sync_channel(1);
    let test_receiver = Arc::new(Mutex::new(test_receiver));
    let server = TestServer::start(move |_req| {
        let (mut sender, response) = Response::event_stream();
        std::thread::spawn(move || {
            let test_receiver_guard = test_receiver.lock().unwrap();
            test_receiver_guard
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            sender.send(Event::Message("msg1".to_string()));
            test_receiver_guard
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            sender.send(Event::custom("type1", "msg2".to_string()).unwrap());
            test_receiver_guard
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
        });
        response
    })
    .unwrap();
    let mut tcp_stream = server.connect_and_send("M / HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n",
    );
    test_sender.send(()).unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "b\r\ndata: msg1\n\r\n"
    );
    test_sender.send(()).unwrap();
    assert_eq!(
        read_for(&mut tcp_stream, 100).unwrap(),
        "18\r\nevent: type1\ndata: msg2\n\r\n"
    );
    test_sender.send(()).unwrap();
    assert_eq!(read_for(&mut tcp_stream, 100).unwrap(), "0\r\n\r\n");
}
