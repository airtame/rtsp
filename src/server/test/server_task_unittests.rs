use super::*;
use crate::server::{Server, ServerHandle};

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn bind_server() -> (ServerHandle, ServerTask) {
    Server::new()
        .bind(std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)))
        .expect("bind failed")
}

#[tokio::test]
async fn await_returns_after_handle_stops_server() {
    let (handle, task) = bind_server();

    handle.stop();

    tokio::time::timeout(TEST_TIMEOUT, task).await.expect("server task did not finish");
}

#[tokio::test]
async fn spawned_task_stops_when_handle_stops_server() {
    let (handle, task) = bind_server();

    let spawned = tokio::spawn(task);
    handle.stop();

    tokio::time::timeout(TEST_TIMEOUT, spawned)
        .await
        .expect("server task did not finish")
        .expect("server task panicked");
}

#[tokio::test]
async fn debug_shows_type_name() {
    let (_handle, task) = bind_server();

    assert_eq!(format!("{task:?}"), "ServerTask");
}
