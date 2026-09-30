use tokio::io::AsyncReadExt;

use super::*;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn localhost_v4() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))
}

fn localhost_v6() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0))
}

async fn run_until_stopped(server: &Server) {
    tokio::time::timeout(TEST_TIMEOUT, server.run())
        .await
        .expect("server run loop did not exit after stop");
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

    tokio::join!(run_until_stopped(&server), async { server.stop() });
}

#[tokio::test]
async fn run_returns_immediately_when_stopped_before_run() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    server.stop();

    run_until_stopped(&server).await;
}

#[tokio::test]
async fn stop_is_idempotent() {
    let server = Server::bind(localhost_v4()).expect("bind failed");

    server.stop();
    server.stop();

    run_until_stopped(&server).await;
}

#[tokio::test]
async fn run_can_be_stopped_from_another_task() {
    let server = std::sync::Arc::new(Server::bind(localhost_v4()).expect("bind failed"));

    let handle = tokio::spawn({
        let server = server.clone();
        async move { server.run().await }
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

    let (_, _clients) = tokio::join!(run_until_stopped(&server), async {
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

    server.accept_connection(&mut connection_tasks, stream, peer_addr);
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

    server.accept_connection(&mut connection_tasks, stream, peer_addr);
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
async fn accept_connection_applies_connection_idle_timeout() {
    let server = Server::bind(localhost_v4())
        .expect("bind failed")
        .with_connection_idle_timeout(std::time::Duration::from_millis(100));
    let mut connection_tasks = tokio::task::JoinSet::new();
    let (_client, stream, peer_addr) = accept_client(&server).await;

    server.accept_connection(&mut connection_tasks, stream, peer_addr);

    let (_, reason) = join_next_connection(&mut connection_tasks).await;
    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
}
