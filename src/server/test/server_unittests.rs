use tokio::io::AsyncReadExt;

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

async fn run_until_stopped(server: &Server, delegate: &impl ServerDelegate) {
    tokio::time::timeout(TEST_TIMEOUT, server.run(delegate))
        .await
        .expect("server run loop did not exit after stop");
}

/// Records every delegate call, and optionally closes each new connection right away.
#[derive(Default)]
struct RecordingDelegate {
    connection_options: ConnectionOptions,
    close_new_connections: bool,
    connection_options_requests: std::sync::Mutex<Vec<std::net::SocketAddr>>,
    new_connections: std::sync::Mutex<Vec<ConnectionHandle>>,
    closed_connections: std::sync::Mutex<Vec<(std::net::SocketAddr, ConnectionCloseReason)>>,
}

impl ServerDelegate for RecordingDelegate {
    fn connection_options(&self, peer_addr: std::net::SocketAddr) -> ConnectionOptions {
        self.connection_options_requests.lock().unwrap().push(peer_addr);
        self.connection_options.clone()
    }

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
    connection_tasks: &mut tokio::task::JoinSet<(std::net::SocketAddr, ConnectionCloseReason)>,
) -> (std::net::SocketAddr, ConnectionCloseReason) {
    tokio::time::timeout(TEST_TIMEOUT, connection_tasks.join_next())
        .await
        .expect("connection task did not finish")
        .expect("no connection task was spawned")
        .expect("connection task panicked")
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

    tokio::join!(run_until_stopped(&server, &()), async { server.stop() });
}

#[tokio::test]
async fn run_returns_immediately_when_stopped_before_run() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    server.stop();

    run_until_stopped(&server, &()).await;
}

#[tokio::test]
async fn stop_is_idempotent() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    server.stop();
    server.stop();

    run_until_stopped(&server, &()).await;
}

#[tokio::test]
async fn run_can_be_stopped_from_another_task() {
    let server = std::sync::Arc::new(Server::bind(localhost_v4()).expect("bind failed"));

    let handle = tokio::spawn({
        let server = server.clone();
        async move { server.run(&()).await }
    });

    server.stop();

    tokio::time::timeout(TEST_TIMEOUT, handle)
        .await
        .expect("server run loop did not exit after stop")
        .expect("server run task panicked");
}

#[tokio::test]
async fn run_returns_after_stop_with_open_connections() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let addr = server.listener.local_addr().unwrap();

    let (_, _clients) = tokio::join!(run_until_stopped(&server, &()), async {
        let mut clients = Vec::new();
        for _ in 0..3 {
            clients
                .push(tokio::net::TcpStream::connect(addr).await.expect("client connect failed"));
        }
        server.stop();
        clients
    });
}

#[tokio::test]
async fn accept_connection_reports_peer_address_and_close_reason() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (client, stream, peer_addr) = accept_client(&server).await;
    let client_addr = client.local_addr().unwrap();

    server.accept_connection(&mut connection_tasks, stream, peer_addr, &());
    drop(client);

    let (addr, reason) = join_next_connection(&mut connection_tasks).await;
    assert_eq!(addr, client_addr);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn stop_cancels_and_closes_accepted_connections() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (mut client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(&mut connection_tasks, stream, peer_addr, &());
    server.stop();

    let (_, reason) = join_next_connection(&mut connection_tasks).await;
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);
}

#[tokio::test]
async fn accept_connection_requests_connection_options_for_peer() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let delegate = RecordingDelegate::default();
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(&mut connection_tasks, stream, peer_addr, &delegate);

    let connection_options_requests = delegate.connection_options_requests.lock().unwrap();
    assert_eq!(*connection_options_requests, vec![client.local_addr().unwrap()]);
}

#[tokio::test]
async fn accept_connection_applies_connection_options_from_delegate() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let delegate = RecordingDelegate {
        connection_options: ConnectionOptions::new()
            .with_idle_timeout(std::time::Duration::from_millis(100)),
        ..Default::default()
    };
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (_client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(&mut connection_tasks, stream, peer_addr, &delegate);

    let (_, reason) = join_next_connection(&mut connection_tasks).await;
    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn accept_connection_passes_connection_handle_to_delegate() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let delegate = RecordingDelegate::default();
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(&mut connection_tasks, stream, peer_addr, &delegate);

    let new_connections = delegate.new_connections.lock().unwrap();
    assert_eq!(new_connections.len(), 1);
    assert_eq!(new_connections[0].peer_addr(), client.local_addr().unwrap());
}

#[tokio::test]
async fn closing_connection_handle_closes_only_that_connection() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let delegate = RecordingDelegate::default();
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (mut closed_client, stream, peer_addr) = accept_client(&server).await;
    server.accept_connection(&mut connection_tasks, stream, peer_addr, &delegate);
    let (_open_client, stream, peer_addr) = accept_client(&server).await;
    server.accept_connection(&mut connection_tasks, stream, peer_addr, &delegate);

    delegate.new_connections.lock().unwrap()[0].close();

    let (addr, reason) = join_next_connection(&mut connection_tasks).await;
    assert_eq!(addr, closed_client.local_addr().unwrap());
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, closed_client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);

    assert_eq!(connection_tasks.len(), 1, "the other connection was closed too");
    assert!(!server.cancellation_token.is_cancelled(), "the server was stopped");
}

#[tokio::test]
async fn run_notifies_delegate_about_new_and_closed_connections() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let addr = server.listener.local_addr().unwrap();
    let delegate = RecordingDelegate { close_new_connections: true, ..Default::default() };

    let (_, client_addr) = tokio::join!(run_until_stopped(&server, &delegate), async {
        let mut client = tokio::net::TcpStream::connect(addr).await.expect("client connect failed");

        // The delegate closes every new connection, so EOF means the run loop accepted it.
        let mut buffer = [0u8; 1];
        let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
            .await
            .expect("connection was not closed")
            .expect("client read failed");
        assert_eq!(read, 0);

        server.stop();
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

fn respond_ok(_: &Request) -> Response {
    Response::new(Version::V1, StatusCode::Ok)
}

#[tokio::test]
async fn bind_starts_with_empty_router() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    assert_eq!(format!("{:?}", server.router), "Router { paths: [] }");
}

#[tokio::test]
async fn with_router_uses_given_router() {
    let router = Router::new();
    router.register("/stream1", respond_ok);

    let server = Server::bind(localhost_v4()).expect("bind failed").with_router(router);

    assert!(server.router.get("/stream1").is_some());
}

#[tokio::test]
async fn with_router_sees_routes_registered_afterwards() {
    let router = Router::new();
    let server = Server::bind(localhost_v4()).expect("bind failed").with_router(router.clone());

    router.register("/stream1", respond_ok);
    router.register("/stream2", respond_ok);
    router.unregister("/stream1");

    assert_eq!(format!("{:?}", server.router), r#"Router { paths: ["/stream2"] }"#);
}
