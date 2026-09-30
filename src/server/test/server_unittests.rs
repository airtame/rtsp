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
async fn run_accepts_connections_until_stopped() {
    let server = Server::bind(localhost_v4()).expect("bind failed");
    let addr = server.listener.local_addr().unwrap();

    tokio::join!(run_until_stopped(&server), async {
        for _ in 0..3 {
            tokio::net::TcpStream::connect(addr).await.expect("client connect failed");
        }
        server.stop();
    });
}
