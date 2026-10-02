use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::message::{ParsingMode, Request, Response, StatusCode, Version};

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const OPTIONS_REQUEST: &[u8] = b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n";
const NOT_FOUND_RESPONSE: &str = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";

/// Fields left out of a `let TestConnection { .. }` pattern are dropped right away, so tests
/// that need the client to stay connected bind it as `client: _client`.
struct TestConnection {
    client: tokio::net::TcpStream,
    connection: Connection,
    handle: ConnectionHandle,
    cancellation_token: tokio_util::sync::CancellationToken,
}

async fn connect() -> TestConnection {
    connect_with(Router::new(), ConnectionOptions::default()).await
}

async fn connect_with_router(router: Router) -> TestConnection {
    connect_with(router, ConnectionOptions::default()).await
}

async fn connect_with_options(options: ConnectionOptions) -> TestConnection {
    connect_with(Router::new(), options).await
}

async fn connect_with(router: Router, options: ConnectionOptions) -> TestConnection {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener bind failed");
    let addr = listener.local_addr().unwrap();

    let (client, accepted) = tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
    let client = client.expect("client connect failed");
    let (stream, peer_addr) = accepted.expect("accept failed");

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (connection, handle) =
        Connection::new(stream, peer_addr, cancellation_token.clone(), router, options);

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
async fn debug_shows_peer_address_options_and_router() {
    let options = ConnectionOptions::new().with_idle_timeout(std::time::Duration::from_secs(60));
    let TestConnection { client, connection, .. } =
        connect_with(router_with_stream(), options).await;

    assert_eq!(
        format!("{connection:?}"),
        format!(
            "Connection {{ peer_addr: {}, options: ConnectionOptions {{ idle_timeout: Some(60s), \
             parsing_mode: Strict, activity_hook: false }}, \
             router: Router {{ paths: [\"/stream1\"] }}, .. }}",
            client.local_addr().unwrap()
        )
    );
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
    let idle_timeout = std::time::Duration::from_millis(100);
    let TestConnection { client: _client, connection, .. } =
        connect_with_options(ConnectionOptions::new().with_idle_timeout(idle_timeout)).await;

    let started = std::time::Instant::now();
    let reason = run_until_closed(connection).await;

    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
    assert!(started.elapsed() >= idle_timeout, "closed before the idle timeout elapsed");
}

#[tokio::test]
async fn run_resets_idle_timeout_when_client_sends_messages() {
    let idle_timeout = std::time::Duration::from_millis(500);
    let TestConnection { mut client, connection, .. } =
        connect_with_options(ConnectionOptions::new().with_idle_timeout(idle_timeout)).await;

    // Keeps sending messages for longer than the idle timeout, with gaps well below it.
    let (reason, _) = tokio::join!(run_until_closed(connection), async {
        for _ in 0..8 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
            read_response(&mut client, NOT_FOUND_RESPONSE.len()).await;
        }
        drop(client);
    });

    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

type ActivityCalls = std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>;

fn options_recording_activity(calls: &ActivityCalls) -> ConnectionOptions {
    let calls = calls.clone();
    ConnectionOptions::new()
        .with_activity_hook(move |peer_addr| calls.lock().unwrap().push(peer_addr))
}

#[tokio::test]
async fn run_calls_activity_hook_with_peer_address_for_each_message() {
    let calls = ActivityCalls::default();
    let TestConnection { mut client, connection, .. } =
        connect_with_options(options_recording_activity(&calls)).await;
    let client_addr = client.local_addr().unwrap();

    tokio::join!(run_until_closed(connection), async {
        for _ in 0..2 {
            client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
            read_response(&mut client, NOT_FOUND_RESPONSE.len()).await;
        }
        drop(client);
    });

    assert_eq!(*calls.lock().unwrap(), vec![client_addr, client_addr]);
}

#[tokio::test]
async fn run_calls_activity_hook_for_malformed_message() {
    let calls = ActivityCalls::default();
    let TestConnection { mut client, connection, .. } =
        connect_with_options(options_recording_activity(&calls)).await;
    let bad_request = "RTSP/1.0 400 Bad Request\r\nCSeq: 1\r\n\r\n";

    tokio::join!(run_until_closed(connection), async {
        client.write_all(b"OPTIONS\r\nCSeq: 1\r\n\r\n").await.expect("client write failed");
        read_response(&mut client, bad_request.len()).await;
        drop(client);
    });

    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn run_does_not_call_activity_hook_for_incomplete_message() {
    let calls = ActivityCalls::default();
    let options =
        options_recording_activity(&calls).with_idle_timeout(std::time::Duration::from_millis(100));
    let TestConnection { mut client, connection, .. } = connect_with_options(options).await;

    client.write_all(b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n").await.expect("client write failed");
    let reason = run_until_closed(connection).await;

    assert!(matches!(reason, ConnectionCloseReason::IdleTimeout), "unexpected reason: {reason:?}");
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn run_keeps_connection_open_after_valid_message() {
    let TestConnection { mut client, connection, .. } = connect().await;

    let (reason, _) = tokio::join!(run_until_closed(connection), async {
        client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
        read_response(&mut client, NOT_FOUND_RESPONSE.len()).await;
        drop(client);
    });

    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_keeps_connection_open_after_valid_response() {
    let TestConnection { mut client, connection, .. } = connect().await;

    client.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\n").await.expect("client write failed");
    drop(client);

    let reason = run_until_closed(connection).await;
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_answers_malformed_start_line_with_bad_request_and_stays_open() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let bad_request = "RTSP/1.0 400 Bad Request\r\nCSeq: 1\r\n\r\n";
    let not_found = "RTSP/1.0 404 Not Found\r\nCSeq: 2\r\n\r\n";

    let (reason, responses) = tokio::join!(run_until_closed(connection), async {
        client.write_all(b"OPTIONS\r\nCSeq: 1\r\n\r\n").await.expect("client write failed");
        let first = read_response(&mut client, bad_request.len()).await;
        client
            .write_all(b"OPTIONS * RTSP/1.0\r\nCSeq: 2\r\n\r\n")
            .await
            .expect("client write failed");
        let second = read_response(&mut client, not_found.len()).await;
        drop(client);
        (first, second)
    });

    assert_eq!(responses, (bad_request.to_owned(), not_found.to_owned()));
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_returns_invalid_message_when_client_sends_malformed_header() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let bad_request = "RTSP/1.0 400 Bad Request\r\n\r\n";

    client.write_all(b"OPTIONS * RTSP/1.0\r\nCSeq 1\r\n\r\n").await.expect("client write failed");

    let reason = run_until_closed(connection).await;
    assert!(
        matches!(&reason, ConnectionCloseReason::InvalidMessage(MessageError::InvalidHeader(line)) if line == "CSeq 1"),
        "unexpected reason: {reason:?}"
    );
    assert_eq!(read_response(&mut client, bad_request.len()).await, bad_request);
}

#[tokio::test]
async fn run_answers_request_without_cseq_with_bad_request_in_strict_mode() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let bad_request = "RTSP/1.0 400 Bad Request\r\n\r\n";

    let (reason, response) = tokio::join!(run_until_closed(connection), async {
        client.write_all(b"OPTIONS * RTSP/1.0\r\n\r\n").await.expect("client write failed");
        let response = read_response(&mut client, bad_request.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, bad_request);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_answers_other_protocol_version_with_bad_request_in_strict_mode() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let bad_request = "RTSP/1.0 400 Bad Request\r\nCSeq: 1\r\n\r\n";

    let (reason, response) = tokio::join!(run_until_closed(connection), async {
        client
            .write_all(b"GET /health HTTP/1.1\r\nCSeq: 1\r\n\r\n")
            .await
            .expect("client write failed");
        let response = read_response(&mut client, bad_request.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, bad_request);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_routes_request_without_cseq_in_lenient_mode() {
    let options = ConnectionOptions::new().with_parsing_mode(ParsingMode::Lenient);
    let TestConnection { mut client, connection, .. } = connect_with_options(options).await;
    let not_found = "RTSP/1.0 404 Not Found\r\n\r\n";

    let (reason, response) = tokio::join!(run_until_closed(connection), async {
        client.write_all(b"OPTIONS * RTSP/1.0\r\n\r\n").await.expect("client write failed");
        let response = read_response(&mut client, not_found.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, not_found);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_routes_other_protocol_version_in_lenient_mode() {
    let options = ConnectionOptions::new().with_parsing_mode(ParsingMode::Lenient);
    let TestConnection { mut client, connection, .. } = connect_with_options(options).await;
    let not_found = "HTTP/1.1 404 Not Found\r\n\r\n";

    let (reason, response) = tokio::join!(run_until_closed(connection), async {
        client.write_all(b"GET /health HTTP/1.1\r\n\r\n").await.expect("client write failed");
        let response = read_response(&mut client, not_found.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, not_found);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

async fn read_response(client: &mut tokio::net::TcpStream, length: usize) -> String {
    let mut received = vec![0; length];
    tokio::time::timeout(TEST_TIMEOUT, client.read_exact(&mut received))
        .await
        .expect("no response received")
        .expect("client read failed");

    String::from_utf8(received).expect("response should be UTF-8")
}

fn router_with_stream() -> Router {
    let router = Router::new();
    router.register("/stream1", |_: &Request| {
        Response::new(Version::V1, StatusCode::Ok).with_body("v=0\r\n")
    });

    router
}

#[tokio::test]
async fn run_answers_request_with_not_found_without_routes() {
    let TestConnection { mut client, connection, .. } = connect().await;
    let expected = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";

    let (reason, response) = tokio::join!(run_until_closed(connection), async {
        client.write_all(OPTIONS_REQUEST).await.expect("client write failed");
        let response = read_response(&mut client, expected.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, expected);
    assert!(matches!(reason, ConnectionCloseReason::ClosedByPeer), "unexpected reason: {reason:?}");
}

#[tokio::test]
async fn run_answers_request_with_response_of_registered_handler() {
    let TestConnection { mut client, connection, .. } =
        connect_with_router(router_with_stream()).await;
    let expected = "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nv=0\r\n";

    let (_, response) = tokio::join!(run_until_closed(connection), async {
        client
            .write_all(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0\r\nCSeq: 2\r\n\r\n")
            .await
            .expect("client write failed");
        let response = read_response(&mut client, expected.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, expected);
}

#[tokio::test]
async fn run_answers_pipelined_requests_in_order() {
    let TestConnection { mut client, connection, .. } =
        connect_with_router(router_with_stream()).await;
    let requests: &[u8] = b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n\
                            DESCRIBE rtsp://example.com/stream1 RTSP/1.0\r\nCSeq: 2\r\n\r\n";
    let expected = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n\
                    RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nv=0\r\n";

    let (_, response) = tokio::join!(run_until_closed(connection), async {
        client.write_all(requests).await.expect("client write failed");
        let response = read_response(&mut client, expected.len()).await;
        drop(client);
        response
    });

    assert_eq!(response, expected);
}
