use super::*;

fn local_addr() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 5554))
}

#[test]
fn local_addr_returns_address_passed_to_new() {
    let handle = ServerHandle::new(local_addr(), tokio_util::sync::CancellationToken::new());

    assert_eq!(handle.local_addr(), local_addr());
}

#[test]
fn stop_cancels_server_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ServerHandle::new(local_addr(), cancellation_token.clone());

    handle.stop();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn stop_on_clone_cancels_server_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ServerHandle::new(local_addr(), cancellation_token.clone());

    handle.clone().stop();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn stop_is_idempotent() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ServerHandle::new(local_addr(), cancellation_token.clone());

    handle.stop();
    handle.stop();

    assert!(cancellation_token.is_cancelled());
}
