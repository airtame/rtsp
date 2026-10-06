use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::connection::ConnectionHandle;
use crate::message::{ParsingMode, Request, Response, StatusCode, Version};

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn localhost_v4() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))
}

fn localhost_v6() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0))
}

async fn run_until_stopped(task: ServerTask) {
    tokio::time::timeout(TEST_TIMEOUT, task)
        .await
        .expect("server run loop did not exit after stop");
}

async fn wait_until_closed_by_server(client: &mut tokio::net::TcpStream) {
    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);
}

struct ClosingDelegate;

impl ServerDelegate for ClosingDelegate {
    fn on_new_connection(&self, connection: ConnectionHandle) {
        connection.close();
    }
}

#[test]
fn new_uses_default_connection_options() {
    assert_eq!(
        format!("{:?}", Server::new()),
        "Server { connection_options: ConnectionOptions { idle_timeout: None, \
         parsing_mode: Strict, activity_hook: false }, .. }"
    );
}

#[test]
fn with_connection_options_replaces_connection_options() {
    let server = Server::new().with_connection_options(
        ConnectionOptions::new()
            .with_idle_timeout(std::time::Duration::from_secs(30))
            .with_parsing_mode(ParsingMode::Lenient),
    );

    assert_eq!(
        format!("{server:?}"),
        "Server { connection_options: ConnectionOptions { idle_timeout: Some(30s), \
         parsing_mode: Lenient, activity_hook: false }, .. }"
    );
}

#[tokio::test]
async fn bind_ipv4_listens_on_requested_address() {
    let (handle, _task) = Server::new().bind(localhost_v4()).expect("bind failed");

    assert_eq!(handle.local_addr().ip(), std::net::Ipv4Addr::LOCALHOST);
    assert_ne!(handle.local_addr().port(), 0);
}

#[tokio::test]
async fn bind_ipv6_listens_on_requested_address() {
    // Some CI containers have no IPv6 loopback; skip rather than fail there.
    if std::net::TcpListener::bind(localhost_v6()).is_err() {
        eprintln!("skipping: IPv6 loopback not available");
        return;
    }

    let (handle, _task) = Server::new().bind(localhost_v6()).expect("bind failed");

    assert_eq!(handle.local_addr().ip(), std::net::Ipv6Addr::LOCALHOST);
    assert_ne!(handle.local_addr().port(), 0);
}

#[tokio::test]
async fn bind_fails_when_address_in_use() {
    let (first, _task) = Server::new().bind(localhost_v4()).expect("bind failed");

    let err =
        Server::new().bind(first.local_addr()).expect_err("second bind to same address succeeded");

    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
}

#[tokio::test]
async fn bind_can_be_called_again_with_same_configuration() {
    let server = Server::new();

    let (first, _first_task) = server.bind(localhost_v4()).expect("first bind failed");
    let (second, _second_task) = server.bind(localhost_v4()).expect("second bind failed");

    assert_ne!(first.local_addr(), second.local_addr());
}

#[tokio::test]
async fn with_delegate_passes_new_connections_to_delegate() {
    let (handle, task) =
        Server::new().with_delegate(ClosingDelegate).bind(localhost_v4()).expect("bind failed");

    tokio::join!(run_until_stopped(task), async {
        let mut client = tokio::net::TcpStream::connect(handle.local_addr())
            .await
            .expect("client connect failed");

        wait_until_closed_by_server(&mut client).await;
        handle.stop();
    });
}

#[tokio::test]
async fn with_connection_options_applies_options_to_accepted_connections() {
    let (handle, task) = Server::new()
        .with_connection_options(
            ConnectionOptions::new().with_idle_timeout(std::time::Duration::from_millis(100)),
        )
        .bind(localhost_v4())
        .expect("bind failed");

    tokio::join!(run_until_stopped(task), async {
        let mut client = tokio::net::TcpStream::connect(handle.local_addr())
            .await
            .expect("client connect failed");

        wait_until_closed_by_server(&mut client).await;
        handle.stop();
    });
}

const DESCRIBE_REQUEST: &str = "DESCRIBE rtsp://example.com/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n";
const OK_RESPONSE: &str = "RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\n";
const NOT_FOUND_RESPONSE: &str = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";

fn respond_ok(_: &Request) -> Response {
    Response::new(Version::V1, StatusCode::Ok)
}

async fn exchange(server: &Server, request: &str, response_length: usize) -> String {
    let (handle, task) = server.bind(localhost_v4()).expect("bind failed");

    let (_, response) = tokio::join!(run_until_stopped(task), async {
        let mut client = tokio::net::TcpStream::connect(handle.local_addr())
            .await
            .expect("client connect failed");

        client.write_all(request.as_bytes()).await.expect("client write failed");
        let mut response = vec![0; response_length];
        tokio::time::timeout(TEST_TIMEOUT, client.read_exact(&mut response))
            .await
            .expect("no response received")
            .expect("client read failed");

        handle.stop();
        response
    });

    String::from_utf8(response).expect("response should be UTF-8")
}

#[tokio::test]
async fn new_answers_requests_with_not_found() {
    let response = exchange(&Server::new(), DESCRIBE_REQUEST, NOT_FOUND_RESPONSE.len()).await;

    assert_eq!(response, NOT_FOUND_RESPONSE);
}

#[tokio::test]
async fn with_handler_answers_requests_with_given_handler() {
    let server = Server::new().with_handler(respond_ok);

    let response = exchange(&server, DESCRIBE_REQUEST, OK_RESPONSE.len()).await;

    assert_eq!(response, OK_RESPONSE);
}

#[tokio::test]
async fn with_handler_sees_routes_registered_on_router_afterwards() {
    let router = Router::new();
    let server = Server::new().with_handler(router.clone());

    router.register("/stream1", respond_ok);

    let response = exchange(&server, DESCRIBE_REQUEST, OK_RESPONSE.len()).await;

    assert_eq!(response, OK_RESPONSE);
}

#[tokio::test]
async fn with_handler_sees_routes_unregistered_from_router_afterwards() {
    let router = Router::new();
    router.register("/stream1", respond_ok);
    let server = Server::new().with_handler(router.clone());

    router.unregister("/stream1");

    let response = exchange(&server, DESCRIBE_REQUEST, NOT_FOUND_RESPONSE.len()).await;

    assert_eq!(response, NOT_FOUND_RESPONSE);
}
