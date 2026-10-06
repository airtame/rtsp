use super::*;
use crate::connection::RequestError;
use crate::message::{RequestMethod, Response, StatusCode, Version};

fn peer_addr() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 5554))
}

fn handle_with_token(cancellation_token: tokio_util::sync::CancellationToken) -> ConnectionHandle {
    let (request_tx, _) = tokio::sync::mpsc::unbounded_channel();
    ConnectionHandle::new(peer_addr(), cancellation_token, request_tx)
}

fn handle_with_channel() -> (ConnectionHandle, tokio::sync::mpsc::UnboundedReceiver<PendingRequest>)
{
    let (request_tx, request_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle =
        ConnectionHandle::new(peer_addr(), tokio_util::sync::CancellationToken::new(), request_tx);

    (handle, request_rx)
}

fn options_request() -> Request {
    Request::new(RequestMethod::Options, "*", Version::V1)
}

#[test]
fn peer_addr_returns_address_passed_to_new() {
    let handle = handle_with_token(tokio_util::sync::CancellationToken::new());

    assert_eq!(handle.peer_addr(), peer_addr());
}

#[test]
fn close_cancels_connection_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = handle_with_token(cancellation_token.clone());

    handle.close();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn close_on_clone_cancels_connection_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = handle_with_token(cancellation_token.clone());

    handle.clone().close();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn is_closed_is_false_for_new_handle() {
    let handle = handle_with_token(tokio_util::sync::CancellationToken::new());

    assert!(!handle.is_closed());
}

#[test]
fn is_closed_is_true_after_close() {
    let handle = handle_with_token(tokio_util::sync::CancellationToken::new());

    handle.close();

    assert!(handle.is_closed());
}

#[test]
fn is_closed_is_true_on_every_clone_after_close() {
    let handle = handle_with_token(tokio_util::sync::CancellationToken::new());
    let clone = handle.clone();

    handle.close();

    assert!(clone.is_closed());
}

#[test]
fn is_closed_is_true_when_parent_token_is_cancelled() {
    let parent_token = tokio_util::sync::CancellationToken::new();
    let handle = handle_with_token(parent_token.child_token());

    parent_token.cancel();

    assert!(handle.is_closed());
}

#[test]
fn send_queues_request_without_being_awaited() {
    let (handle, mut request_rx) = handle_with_channel();

    let _response = handle.send(options_request());

    let pending = request_rx.try_recv().expect("request should be queued");
    assert_eq!(pending.request.method(), &RequestMethod::Options);
}

#[tokio::test]
async fn send_returns_response_from_connection() {
    let (handle, mut request_rx) = handle_with_channel();

    let (response, _) = tokio::join!(handle.send(options_request()), async {
        let pending = request_rx.recv().await.expect("request should be queued");
        assert_eq!(pending.request.method(), &RequestMethod::Options);
        pending
            .response_tx
            .send(Response::new(Version::V1, StatusCode::Ok))
            .expect("caller should be waiting");
    });

    assert_eq!(response.expect("send should succeed").status_code(), 200);
}

#[tokio::test]
async fn send_fails_when_connection_is_gone() {
    let (handle, request_rx) = handle_with_channel();
    drop(request_rx);

    let result = handle.send(options_request()).await;

    assert!(matches!(result, Err(RequestError::ConnectionClosed)), "unexpected result: {result:?}");
}

#[tokio::test]
async fn send_fails_when_connection_drops_request() {
    let (handle, mut request_rx) = handle_with_channel();

    let (result, _) = tokio::join!(handle.send(options_request()), async {
        drop(request_rx.recv().await.expect("request should be queued"));
    });

    assert!(matches!(result, Err(RequestError::ConnectionClosed)), "unexpected result: {result:?}");
}

#[test]
fn debug_shows_peer_address_and_state() {
    let handle = handle_with_token(tokio_util::sync::CancellationToken::new());

    assert_eq!(
        format!("{handle:?}"),
        "ConnectionHandle { peer_addr: 127.0.0.1:5554, is_closed: false, .. }"
    );
}
