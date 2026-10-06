use tokio::io::AsyncReadExt;

use super::*;
use crate::connection::ConnectionHandle;
use crate::router::Router;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

async fn server_loop() -> ServerLoop {
    server_loop_with((), ConnectionOptions::default()).await
}

async fn server_loop_with(
    delegate: impl ServerDelegate + 'static,
    connection_options: ConnectionOptions,
) -> ServerLoop {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener bind failed");

    ServerLoop::new(
        listener,
        tokio_util::sync::CancellationToken::new(),
        std::sync::Arc::new(Router::new()),
        std::sync::Arc::new(delegate),
        connection_options,
    )
}

async fn run_until_stopped(server_loop: ServerLoop) {
    tokio::time::timeout(TEST_TIMEOUT, server_loop.run())
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
    server_loop: &ServerLoop,
) -> (tokio::net::TcpStream, tokio::net::TcpStream, std::net::SocketAddr) {
    let addr = server_loop.listener.local_addr().unwrap();

    let (client, accepted) =
        tokio::join!(tokio::net::TcpStream::connect(addr), server_loop.listener.accept());
    let client = client.expect("client connect failed");
    let (stream, peer_addr) = accepted.expect("accept failed");

    (client, stream, peer_addr)
}

async fn join_next_connection(
    server_loop: &mut ServerLoop,
) -> (std::net::SocketAddr, ConnectionCloseReason) {
    tokio::time::timeout(TEST_TIMEOUT, server_loop.connection_tasks.join_next())
        .await
        .expect("connection task did not finish")
        .expect("no connection task was spawned")
        .expect("connection task panicked")
}

async fn next_event(server_loop: &mut ServerLoop) -> ServerEvent {
    tokio::time::timeout(TEST_TIMEOUT, server_loop.next_event()).await.expect("no server event")
}

#[tokio::test]
async fn run_returns_after_stop() {
    let server_loop = server_loop().await;
    let cancellation_token = server_loop.cancellation_token.clone();

    tokio::join!(run_until_stopped(server_loop), async { cancellation_token.cancel() });
}

#[tokio::test]
async fn run_returns_immediately_when_stopped_before_run() {
    let server_loop = server_loop().await;

    server_loop.cancellation_token.cancel();

    run_until_stopped(server_loop).await;
}

#[tokio::test]
async fn run_returns_after_stop_with_open_connections() {
    let server_loop = server_loop().await;
    let addr = server_loop.listener.local_addr().unwrap();
    let cancellation_token = server_loop.cancellation_token.clone();

    let (_, _clients) = tokio::join!(run_until_stopped(server_loop), async {
        let mut clients = Vec::new();
        for _ in 0..3 {
            clients
                .push(tokio::net::TcpStream::connect(addr).await.expect("client connect failed"));
        }
        cancellation_token.cancel();
        clients
    });
}

#[tokio::test]
async fn run_notifies_delegate_about_new_and_closed_connections() {
    let delegate = RecordingDelegate { close_new_connections: true, ..Default::default() };
    let server_loop = server_loop_with(delegate.clone(), ConnectionOptions::default()).await;
    let addr = server_loop.listener.local_addr().unwrap();
    let cancellation_token = server_loop.cancellation_token.clone();

    let (_, client_addr) = tokio::join!(run_until_stopped(server_loop), async {
        let mut client = tokio::net::TcpStream::connect(addr).await.expect("client connect failed");

        // The delegate closes every new connection, so EOF means the run loop accepted it.
        let mut buffer = [0u8; 1];
        let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
            .await
            .expect("connection was not closed")
            .expect("client read failed");
        assert_eq!(read, 0);

        cancellation_token.cancel();
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

#[tokio::test]
async fn next_event_returns_cancelled_after_stop() {
    let mut server_loop = server_loop().await;

    server_loop.cancellation_token.cancel();

    let event = next_event(&mut server_loop).await;
    assert!(matches!(event, ServerEvent::Cancelled), "unexpected event: {event:?}");
}

#[tokio::test]
async fn next_event_returns_connection_accepted_for_new_client() {
    let mut server_loop = server_loop().await;
    let addr = server_loop.listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(addr).await.expect("client connect failed");
    let client_addr = client.local_addr().unwrap();

    let event = next_event(&mut server_loop).await;

    assert!(
        matches!(event, ServerEvent::ConnectionAccepted(_, peer_addr) if peer_addr == client_addr),
        "unexpected event: {event:?}"
    );
}

#[tokio::test]
async fn next_event_returns_connection_closed_when_connection_ends() {
    let mut server_loop = server_loop().await;
    let (client, stream, peer_addr) = accept_client(&server_loop).await;
    let client_addr = client.local_addr().unwrap();
    server_loop.accept_connection(stream, peer_addr);

    drop(client);

    let event = next_event(&mut server_loop).await;
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
    let mut server_loop = server_loop().await;
    let (client, stream, peer_addr) = accept_client(&server_loop).await;
    let client_addr = client.local_addr().unwrap();

    server_loop.accept_connection(stream, peer_addr);
    drop(client);

    let (addr, reason) = join_next_connection(&mut server_loop).await;
    assert_eq!(addr, client_addr);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn accept_connection_applies_connection_options() {
    let mut server_loop = server_loop_with(
        (),
        ConnectionOptions::new().with_idle_timeout(std::time::Duration::from_millis(100)),
    )
    .await;
    let (_client, stream, peer_addr) = accept_client(&server_loop).await;

    server_loop.accept_connection(stream, peer_addr);

    let (_, reason) = join_next_connection(&mut server_loop).await;
    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn accept_connection_passes_connection_handle_to_delegate() {
    let delegate = RecordingDelegate::default();
    let mut server_loop = server_loop_with(delegate.clone(), ConnectionOptions::default()).await;
    let (client, stream, peer_addr) = accept_client(&server_loop).await;

    server_loop.accept_connection(stream, peer_addr);

    let new_connections = delegate.new_connections.lock().unwrap();
    assert_eq!(new_connections.len(), 1);
    assert_eq!(new_connections[0].peer_addr(), client.local_addr().unwrap());
}

#[tokio::test]
async fn cancelling_server_closes_accepted_connections() {
    let mut server_loop = server_loop().await;
    let (mut client, stream, peer_addr) = accept_client(&server_loop).await;

    server_loop.accept_connection(stream, peer_addr);
    server_loop.cancellation_token.cancel();

    let (_, reason) = join_next_connection(&mut server_loop).await;
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);
}

#[tokio::test]
async fn closing_connection_handle_closes_only_that_connection() {
    let delegate = RecordingDelegate::default();
    let mut server_loop = server_loop_with(delegate.clone(), ConnectionOptions::default()).await;
    let (mut closed_client, stream, peer_addr) = accept_client(&server_loop).await;
    server_loop.accept_connection(stream, peer_addr);
    let (_open_client, stream, peer_addr) = accept_client(&server_loop).await;
    server_loop.accept_connection(stream, peer_addr);

    delegate.new_connections.lock().unwrap()[0].close();

    let (addr, reason) = join_next_connection(&mut server_loop).await;
    assert_eq!(addr, closed_client.local_addr().unwrap());
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, closed_client.read(&mut buffer))
        .await
        .expect("connection was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);

    assert_eq!(server_loop.connection_tasks.len(), 1, "the other connection was closed too");
    assert!(!server_loop.cancellation_token.is_cancelled(), "the server was stopped");
}
