use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::connection::{ConnectionHandle, ConnectionOptions};
use crate::message::{Request, Response, StatusCode, Version};

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn localhost_v4() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))
}

fn localhost_v6() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0))
}

async fn run_until_stopped(server: Server) {
    tokio::time::timeout(TEST_TIMEOUT, server.run())
        .await
        .expect("server run loop did not exit after stop");
}

/// Records every delegate call, and optionally closes each new connection right away.
#[derive(Clone, Default)]
struct RecordingDelegate {
    close_new_connections: bool,
    new_connections: std::sync::Arc<std::sync::Mutex<Vec<ConnectionHandle>>>,
    closed_connections:
        std::sync::Arc<std::sync::Mutex<Vec<(std::net::SocketAddr, ConnectionCloseReason)>>>,
}

impl ServerDelegate for RecordingDelegate {
    fn on_new_connection(&self, connection: ConnectionHandle) {
        if self.close_new_connections {
            connection.close();
        }
        self.new_connections.lock().unwrap().push(connection);
    }

    fn on_connection_closed(&self, peer_addr: std::net::SocketAddr, reason: ConnectionCloseReason) {
        self.closed_connections.lock().unwrap().push((peer_addr, reason));
    }
}

/// Connects a client to the server and accepts it, without going through the run loop.
async fn accept_client(
    server: &Server,
) -> (tokio::net::TcpStream, tokio::net::TcpStream, std::net::SocketAddr) {
    let addr = server.listener.local_addr().unwrap();

    let (client, accepted) =
        tokio::join!(tokio::net::TcpStream::connect(addr), server.listener.accept());
    let client = client.expect("client connect failed");
    let (stream, peer_addr) = accepted.expect("accept failed");

    (client, stream, peer_addr)
}

async fn join_next_connection(
    server: &mut Server,
) -> (std::net::SocketAddr, ConnectionCloseReason) {
    tokio::time::timeout(TEST_TIMEOUT, server.connection_tasks.join_next())
        .await
        .expect("connection task did not finish")
        .expect("no connection task was spawned")
        .expect("connection task panicked")
}

async fn next_event(server: &mut Server) -> ServerEvent {
    tokio::time::timeout(TEST_TIMEOUT, server.next_event()).await.expect("no server event")
}

#[tokio::test]
async fn bind_ipv4_listens_on_requested_address() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let local_addr = server.listener.local_addr().unwrap();

    assert_eq!(local_addr.ip(), std::net::Ipv4Addr::LOCALHOST);
    assert_ne!(local_addr.port(), 0);
}

#[tokio::test]
async fn bind_ipv6_listens_on_requested_address() {
    // Some CI containers have no IPv6 loopback; skip rather than fail there.
    if std::net::TcpListener::bind(localhost_v6()).is_err() {
        eprintln!("skipping: IPv6 loopback not available");
        return;
    }

    let server = Server::bind(localhost_v6()).expect("bind failed");
    let local_addr = server.listener.local_addr().unwrap();

    assert_eq!(local_addr.ip(), std::net::Ipv6Addr::LOCALHOST);
    assert_ne!(local_addr.port(), 0);
}

#[tokio::test]
async fn bind_fails_when_address_in_use() {
    let first = Server::bind(localhost_v4()).expect("bind failed");
    let addr = first.listener.local_addr().unwrap();

    let err = Server::bind(addr).err().expect("second bind to same address succeeded");

    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
}

#[tokio::test]
async fn run_returns_after_stop() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let handle = server.handle();

    tokio::join!(run_until_stopped(server), async { handle.stop() });
}

#[tokio::test]
async fn run_returns_immediately_when_stopped_before_run() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    server.handle().stop();

    run_until_stopped(server).await;
}

#[tokio::test]
async fn stop_is_idempotent() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let handle = server.handle();

    handle.stop();
    handle.stop();

    run_until_stopped(server).await;
}

#[tokio::test]
async fn run_can_be_stopped_from_another_task() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let handle = server.handle();

    let run = tokio::spawn(server.run());

    handle.stop();

    tokio::time::timeout(TEST_TIMEOUT, run)
        .await
        .expect("server run loop did not exit after stop")
        .expect("server run task panicked");
}

#[tokio::test]
async fn run_returns_after_stop_with_open_connections() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let addr = server.listener.local_addr().unwrap();
    let handle = server.handle();

    let (_, _clients) = tokio::join!(run_until_stopped(server), async {
        let mut clients = Vec::new();
        for _ in 0..3 {
            clients
                .push(tokio::net::TcpStream::connect(addr).await.expect("client connect failed"));
        }
        handle.stop();
        clients
    });
}

#[tokio::test]
async fn next_event_returns_cancelled_after_stop() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed");

    server.handle().stop();

    let event = next_event(&mut server).await;
    assert!(matches!(event, ServerEvent::Cancelled), "unexpected event: {event:?}");
}

#[tokio::test]
async fn next_event_returns_connection_accepted_for_new_client() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed");
    let addr = server.listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(addr).await.expect("client connect failed");
    let client_addr = client.local_addr().unwrap();

    let event = next_event(&mut server).await;

    assert!(
        matches!(event, ServerEvent::ConnectionAccepted(_, peer_addr) if peer_addr == client_addr),
        "unexpected event: {event:?}"
    );
}

#[tokio::test]
async fn next_event_returns_connection_closed_when_connection_ends() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed");
    let (client, stream, peer_addr) = accept_client(&server).await;
    let client_addr = client.local_addr().unwrap();
    server.accept_connection(stream, peer_addr);

    drop(client);

    let event = next_event(&mut server).await;
    assert!(
        matches!(
            event,
            ServerEvent::ConnectionClosed(Ok((addr, ConnectionCloseReason::ClosedByPeer)))
                if addr == client_addr
        ),
        "unexpected event: {event:?}"
    );
}

#[tokio::test]
async fn accept_connection_reports_peer_address_and_close_reason() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed");
    let (client, stream, peer_addr) = accept_client(&server).await;
    let client_addr = client.local_addr().unwrap();

    server.accept_connection(stream, peer_addr);
    drop(client);

    let (addr, reason) = join_next_connection(&mut server).await;
    assert_eq!(addr, client_addr);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn stop_cancels_and_closes_accepted_connections() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed");
    let (mut client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(stream, peer_addr);
    server.handle().stop();

    let (_, reason) = join_next_connection(&mut server).await;
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);
}

#[tokio::test]
async fn with_connection_options_applies_options_to_accepted_connections() {
    let mut server = Server::bind(localhost_v4()).expect("bind failed").with_connection_options(
        ConnectionOptions::new().with_idle_timeout(std::time::Duration::from_millis(100)),
    );
    let (_client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(stream, peer_addr);

    let (_, reason) = join_next_connection(&mut server).await;
    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn with_delegate_passes_connection_handle_to_delegate() {
    let delegate = RecordingDelegate::default();
    let mut server =
        Server::bind(localhost_v4()).expect("bind failed").with_delegate(delegate.clone());
    let (client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(stream, peer_addr);

    let new_connections = delegate.new_connections.lock().unwrap();
    assert_eq!(new_connections.len(), 1);
    assert_eq!(new_connections[0].peer_addr(), client.local_addr().unwrap());
}

#[tokio::test]
async fn closing_connection_handle_closes_only_that_connection() {
    let delegate = RecordingDelegate::default();
    let mut server =
        Server::bind(localhost_v4()).expect("bind failed").with_delegate(delegate.clone());
    let (mut closed_client, stream, peer_addr) = accept_client(&server).await;
    server.accept_connection(stream, peer_addr);
    let (_open_client, stream, peer_addr) = accept_client(&server).await;
    server.accept_connection(stream, peer_addr);

    delegate.new_connections.lock().unwrap()[0].close();

    let (addr, reason) = join_next_connection(&mut server).await;
    assert_eq!(addr, closed_client.local_addr().unwrap());
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, closed_client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);

    assert_eq!(server.connection_tasks.len(), 1, "the other connection was closed too");
    assert!(!server.cancellation_token.is_cancelled(), "the server was stopped");
}

#[tokio::test]
async fn run_notifies_delegate_about_new_and_closed_connections() {
    let delegate = RecordingDelegate { close_new_connections: true, ..Default::default() };
    let server = Server::bind(localhost_v4()).expect("bind failed").with_delegate(delegate.clone());
    let addr = server.listener.local_addr().unwrap();
    let handle = server.handle();

    let (_, client_addr) = tokio::join!(run_until_stopped(server), async {
        let mut client = tokio::net::TcpStream::connect(addr).await.expect("client connect failed");

        // The delegate closes every new connection, so EOF means the run loop accepted it.
        let mut buffer = [0u8; 1];
        let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
            .await
            .expect("connection was not closed")
            .expect("client read failed");
        assert_eq!(read, 0);

        handle.stop();
        client.local_addr().unwrap()
    });

    let new_connections = delegate.new_connections.lock().unwrap();
    assert_eq!(new_connections.len(), 1);
    assert_eq!(new_connections[0].peer_addr(), client_addr);

    let closed_connections = delegate.closed_connections.lock().unwrap();
    assert_eq!(closed_connections.len(), 1);
    assert_eq!(closed_connections[0].0, client_addr);
    assert!(
        matches!(closed_connections[0].1, ConnectionCloseReason::Cancelled),
        "unexpected reason: {:?}",
        closed_connections[0].1
    );
}

const DESCRIBE_REQUEST: &str = "DESCRIBE rtsp://example.com/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n";
const OK_RESPONSE: &str = "RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\n";
const NOT_FOUND_RESPONSE: &str = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";

fn respond_ok(_: &Request) -> Response {
    Response::new(Version::V1, StatusCode::Ok)
}

async fn exchange(mut server: Server, request: &str, response_length: usize) -> String {
    let (mut client, stream, peer_addr) = accept_client(&server).await;
    server.accept_connection(stream, peer_addr);

    client.write_all(request.as_bytes()).await.expect("client write failed");
    let mut response = vec![0; response_length];
    tokio::time::timeout(TEST_TIMEOUT, client.read_exact(&mut response))
        .await
        .expect("no response received")
        .expect("client read failed");

    String::from_utf8(response).expect("response should be UTF-8")
}

#[tokio::test]
async fn bind_answers_requests_with_not_found() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    let response = exchange(server, DESCRIBE_REQUEST, NOT_FOUND_RESPONSE.len()).await;

    assert_eq!(response, NOT_FOUND_RESPONSE);
}

#[tokio::test]
async fn with_handler_answers_requests_with_given_handler() {
    let server = Server::bind(localhost_v4()).expect("bind failed").with_handler(respond_ok);

    let response = exchange(server, DESCRIBE_REQUEST, OK_RESPONSE.len()).await;

    assert_eq!(response, OK_RESPONSE);
}

#[tokio::test]
async fn with_handler_sees_routes_registered_on_router_afterwards() {
    let router = Router::new();
    let server = Server::bind(localhost_v4()).expect("bind failed").with_handler(router.clone());

    router.register("/stream1", respond_ok);

    let response = exchange(server, DESCRIBE_REQUEST, OK_RESPONSE.len()).await;

    assert_eq!(response, OK_RESPONSE);
}

#[tokio::test]
async fn with_handler_sees_routes_unregistered_from_router_afterwards() {
    let router = Router::new();
    router.register("/stream1", respond_ok);
    let server = Server::bind(localhost_v4()).expect("bind failed").with_handler(router.clone());

    router.unregister("/stream1");

    let response = exchange(server, DESCRIBE_REQUEST, NOT_FOUND_RESPONSE.len()).await;

    assert_eq!(response, NOT_FOUND_RESPONSE);
}
