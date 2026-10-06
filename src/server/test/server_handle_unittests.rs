use super::*;

#[test]
fn stop_cancels_server_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ServerHandle::new(cancellation_token.clone());

    handle.stop();

    assert!(cancellation_token.is_cancelled());
}

#[test]
fn stop_on_clone_cancels_server_token() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let handle = ServerHandle::new(cancellation_token.clone());

    handle.clone().stop();

    assert!(cancellation_token.is_cancelled());
}
