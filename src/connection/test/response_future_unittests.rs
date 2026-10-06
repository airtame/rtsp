use super::*;
use crate::message::{StatusCode, Version};

#[tokio::test]
async fn resolves_to_response_sent_by_connection() {
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    let future = ResponseFuture::new(response_rx);

    response_tx.send(Response::new(Version::V1, StatusCode::Ok)).expect("future should be waiting");

    assert_eq!(future.await.expect("future should resolve").status_code(), 200);
}

#[tokio::test]
async fn resolves_to_connection_closed_when_connection_drops_request() {
    let (response_tx, response_rx) = tokio::sync::oneshot::channel::<Response>();
    let future = ResponseFuture::new(response_rx);

    drop(response_tx);

    let result = future.await;
    assert!(matches!(result, Err(RequestError::ConnectionClosed)), "unexpected result: {result:?}");
}

#[test]
fn debug_shows_type_name() {
    let (_response_tx, response_rx) = tokio::sync::oneshot::channel();

    assert_eq!(format!("{:?}", ResponseFuture::new(response_rx)), "ResponseFuture");
}
