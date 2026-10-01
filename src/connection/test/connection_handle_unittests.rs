use super::*;

fn peer_addr() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 5554))
}

#[test]
fn peer_addr_returns_address_passed_to_new() {
    let handle = ConnectionHandle::new(peer_addr(), tokio_util::sync::CancellationToken::new());

    assert_eq!(handle.peer_addr(), peer_addr());
}

#[test]
fn close_cancels_connection_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ConnectionHandle::new(peer_addr(), cancellation_token.clone());

    handle.close();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn close_on_clone_cancels_connection_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ConnectionHandle::new(peer_addr(), cancellation_token.clone());

    handle.clone().close();

    assert!(cancellation_token.is_cancelled());
}
