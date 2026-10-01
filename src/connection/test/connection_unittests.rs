use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const OPTIONS_REQUEST: &[u8] = b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n";

/// Fields left out of a `let TestConnection { .. }` pattern are dropped right away, so tests
/// that need the client to stay connected bind it as `client: _client`.
struct TestConnection {
    client: tokio::net::TcpStream,
    connection: Connection,
    handle: ConnectionHandle,
    cancellation_token: tokio_util::sync::CancellationToken,
}

async fn connect() -> TestConnection {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener bind failed");
    let addr = listener.local_addr().unwrap();

    let (client, accepted) = tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
    let client = client.expect("client connect failed");
    let (stream, peer_addr) = accepted.expect("accept failed");

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (connection, handle) = Connection::new(stream, peer_addr, cancellation_token.clone());

    TestConnection { client, connection, handle, cancellation_token }
}

async fn run_until_closed(connection: Connection) -> ConnectionCloseReason {
    tokio::time::timeout(TEST_TIMEOUT, connection.run())
        .await
        .expect("connection run loop did not exit")
}

#[tokio::test]
async fn peer_addr_returns_client_address() {
    let TestConnection { client, connection, .. } = connect().await;

    assert_eq!(connection.peer_addr(), client.local_addr().unwrap());
}

#[tokio::test]
async fn new_returns_handle_for_same_peer() {
    let TestConnection { client, handle, .. } = connect().await;

    assert_eq!(handle.peer_addr(), client.local_addr().unwrap());
}

#[tokio::test]
async fn run_returns_closed_by_peer_when_client_disconnects() {
    let TestConnection { client, connection, .. } = connect().await;

    drop(client);

    let reason = run_until_closed(connection).await;
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_returns_cancelled_when_token_is_cancelled() {
    let TestConnection { client: _client, connection, cancellation_token, .. } = connect().await;

    let (reason, _) =
        tokio::join!(run_until_closed(connection), async { cancellation_token.cancel() });

    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_returns_cancelled_when_handle_is_closed() {
    let TestConnection { client: _client, connection, handle, .. } = connect().await;

    let (reason, _) = tokio::join!(run_until_closed(connection), async { handle.close() });

    assert!(matches!(reason, ConnectionCloseReason::Cancelled), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_closes_stream_when_cancelled() {
    let TestConnection { mut client, connection, cancellation_token, .. } = connect().await;

    cancellation_token.cancel();
    run_until_closed(connection).await;

    let mut buffer = [0u8; 1];
    let read = tokio::time::timeout(TEST_TIMEOUT, client.read(&mut buffer))
        .await
        .expect("stream was not closed")
        .expect("client read failed");
    assert_eq!(read, 0);
}

#[tokio::test]
async fn run_returns_io_error_when_connection_is_reset() {
    let TestConnection { client, connection, .. } = connect().await;

    // Dropping a socket with a zero linger timeout sends a TCP RST instead of a FIN.
    client.set_zero_linger().expect("set_zero_linger failed");
    drop(client);

    let reason = run_until_closed(connection).await;
    assert!(
        matches!(&reason, ConnectionCloseReason::Io(err) if err.kind() == std::io::ErrorKind::ConnectionReset),
        "unexpected reason: {reason:?}"
    );
}

#[tokio::test]
async fn run_returns_idle_timeout_when_client_sends_nothing() {
    let TestConnection { client: _client, connection, .. } = connect().await;
    let idle_timeout = std::time::Duration::from_millis(100);

    let started = std::time::Instant::now();
    let reason = run_until_closed(connection.with_idle_timeout(idle_timeout)).await;

    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
    assert!(started.elapsed() >= idle_timeout, "closed before the idle timeout elapsed");
}

#[tokio::test]
async fn run_resets_idle_timeout_when_client_sends_messages() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let connection = connection.with_idle_timeout(std::time::Duration::from_millis(500));

    // Keeps sending messages for longer than the idle timeout, with gaps well below it.
    let (reason, _) = tokio::join!(run_until_closed(connection), async {
        for _ in 0..8 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
        }
        drop(client);
    });

    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_keeps_connection_open_after_valid_message() {
    let TestConnection { mut client, connection, .. } = connect().await;

    client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
    drop(client);

    let reason = run_until_closed(connection).await;
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_returns_invalid_message_when_client_sends_malformed_header() {
    let TestConnection { mut client, connection, .. } = connect().await;

    client.write_all(b"OPTIONS * RTSP/1.0\r\nCSeq 1\r\n\r\n").await.expect("client write failed");

    let reason = run_until_closed(connection).await;
    assert!(
        matches!(&reason, ConnectionCloseReason::InvalidMessage(MessageError::InvalidHeader(line)) if line == "CSeq 1"),
        "unexpected reason: {reason:?}"
    );
}
