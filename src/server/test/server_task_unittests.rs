use super::*;
use crate::server::ServerHandle;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn run_server() -> (ServerHandle, ServerTask) {
    Server::bind(std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)))
        .expect("bind failed")
        .run()
}

#[tokio::test]
async fn spawned_task_stops_when_handle_stops_server() {
    let (handle, task) = run_server();

    let spawned = tokio::spawn(task);
    handle.stop();

    tokio::time::timeout(TEST_TIMEOUT, spawned)
        .await
        .expect("server task did not finish")
        .expect("server task panicked");
}

#[tokio::test]
async fn debug_shows_type_name() {
    let (_handle, task) = run_server();

    assert_eq!(format!("{task:?}"), "ServerTask");
}
