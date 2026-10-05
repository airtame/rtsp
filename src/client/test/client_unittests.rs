use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::connection::ConnectionCloseReason;
use crate::message::{ParsingMode, Request, Response, StatusCode, Version};

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

struct TestConnection {
    server: tokio::net::TcpStream,
    handle: ConnectionHandle,
    task: ConnectionTask,
}

async fn connect(client: &Client) -> TestConnection {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener bind failed");
    let addr = listener.local_addr().unwrap();

    let (connected, accepted) = tokio::join!(client.connect(addr), listener.accept());
    let (handle, task) = connected.expect("client connect failed");
    let (server, _) = accepted.expect("accept failed");

    TestConnection { server, handle, task }
}

async fn run_until_closed(task: ConnectionTask) -> ConnectionCloseReason {
    tokio::time::timeout(TEST_TIMEOUT, task).await.expect("connection task did not finish")
}

async fn read_response(server: &mut tokio::net::TcpStream, length: usize) -> String {
    let mut received = vec![0; length];
    tokio::time::timeout(TEST_TIMEOUT, server.read_exact(&mut received))
        .await
        .expect("no response received")
        .expect("server read failed");

    String::from_utf8(received).expect("response should be UTF-8")
}

#[test]
fn new_uses_default_connection_options() {
    assert_eq!(
        format!("{:?}", Client::new()),
        "Client { connection_options: ConnectionOptions { idle_timeout: None, \
         parsing_mode: Strict, activity_hook: false }, .. }"
    );
}

#[test]
fn with_connection_options_replaces_connection_options() {
    let client = Client::new().with_connection_options(
        ConnectionOptions::new()
            .with_idle_timeout(std::time::Duration::from_secs(30))
            .with_parsing_mode(ParsingMode::Lenient),
    );

    assert_eq!(
        format!("{client:?}"),
        "Client { connection_options: ConnectionOptions { idle_timeout: Some(30s), \
         parsing_mode: Lenient, activity_hook: false }, .. }"
    );
}

#[tokio::test]
async fn connect_returns_handle_for_server_address() {
    let TestConnection { server, handle, .. } = connect(&Client::new()).await;

    assert_eq!(handle.peer_addr(), server.local_addr().unwrap());
}

#[tokio::test]
async fn connect_fails_when_nothing_listens() {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .expect("listener bind failed");
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let err = Client::new().connect(addr).await.expect_err("connect should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::ConnectionRefused);
}

#[tokio::test]
async fn task_returns_closed_by_peer_when_server_disconnects() {
    let TestConnection { server, task, .. } = connect(&Client::new()).await;

    drop(server);

    let reason = run_until_closed(task).await;
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn task_returns_cancelled_when_handle_is_closed() {
    let TestConnection { server: _server, handle, task } = connect(&Client::new()).await;

    handle.close();

    let reason = run_until_closed(task).await;
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn closing_one_connection_keeps_the_others_open() {
    let client = Client::new();
    let TestConnection { server: _closed_server, handle, task: closed_task } =
        connect(&client).await;
    let TestConnection { server: open_server, task: open_task, .. } = connect(&client).await;

    handle.close();
    let closed_reason = run_until_closed(closed_task).await;
    drop(open_server);
    let open_reason = run_until_closed(open_task).await;

    assert!(
        matches!(closed_reason, ConnectionCloseReason::Cancelled),
        "unexpected reason: {closed_reason:?}"
    );
    assert!(
        matches!(open_reason, ConnectionCloseReason::ClosedByPeer),
        "unexpected reason: {open_reason:?}"
    );
}

#[tokio::test]
async fn task_answers_server_request_with_not_found_by_default() {
    let TestConnection { mut server, task, .. } = connect(&Client::new()).await;
    let expected = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";

    let (_, response) = tokio::join!(run_until_closed(task), async {
        server
            .write_all(b"GET_PARAMETER rtsp://example.com/stream RTSP/1.0\r\nCSeq: 1\r\n\r\n")
            .await
            .expect("server write failed");
        let response = read_response(&mut server, expected.len()).await;
        drop(server);
        response
    });

    assert_eq!(response, expected);
}

#[tokio::test]
async fn task_answers_server_request_with_handler() {
    let client =
        Client::new().with_handler(|_: &Request| Response::new(Version::V1, StatusCode::Ok));
    let TestConnection { mut server, task, .. } = connect(&client).await;
    let expected = "RTSP/1.0 200 OK\r\nCSeq: 2\r\n\r\n";

    let (_, response) = tokio::join!(run_until_closed(task), async {
        server
            .write_all(b"GET_PARAMETER rtsp://example.com/stream RTSP/1.0\r\nCSeq: 2\r\n\r\n")
            .await
            .expect("server write failed");
        let response = read_response(&mut server, expected.len()).await;
        drop(server);
        response
    });

    assert_eq!(response, expected);
}

#[tokio::test]
async fn task_applies_connection_options() {
    let client = Client::new().with_connection_options(
        ConnectionOptions::new().with_idle_timeout(std::time::Duration::from_millis(100)),
    );
    let TestConnection { server: _server, task, .. } = connect(&client).await;

    let reason = run_until_closed(task).await;

    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
}
