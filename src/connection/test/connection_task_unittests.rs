use super::*;
use crate::connection::{ConnectionHandle, ConnectionOptions};
use crate::router::Router;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

async fn connect() -> (tokio::net::TcpStream, ConnectionTask, ConnectionHandle) {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener bind failed");
    let addr = listener.local_addr().unwrap();

    let (client, accepted) = tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
    let client = client.expect("client connect failed");
    let (stream, peer_addr) = accepted.expect("accept failed");

    let (connection, handle) = Connection::new(
        stream,
        peer_addr,
        tokio_util::sync::CancellationToken::new(),
        std::sync::Arc::new(Router::new()),
        ConnectionOptions::default(),
    );

    (client, ConnectionTask::new(connection), handle)
}

#[tokio::test]
async fn await_returns_close_reason_of_connection() {
    let (client, task, _handle) = connect().await;

    drop(client);

    let reason =
        tokio::time::timeout(TEST_TIMEOUT, task).await.expect("connection task did not finish");
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn spawned_task_stops_when_handle_is_closed() {
    let (_client, task, handle) = connect().await;

    let spawned = tokio::spawn(task);
    handle.close();

    let reason = tokio::time::timeout(TEST_TIMEOUT, spawned)
        .await
        .expect("connection task did not finish")
        .expect("connection task panicked");
    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn debug_shows_type_name() {
    let (_client, task, _handle) = connect().await;

    assert_eq!(format!("{task:?}"), "ConnectionTask");
}
