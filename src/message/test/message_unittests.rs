use super::*;

fn parse(
    start_line: &[u8],
    header_lines: &str,
    body: &'static [u8],
    parsing_mode: ParsingMode,
) -> Result<Message, MessageError> {
    let headers =
        MessageHeaders::try_from(header_lines.as_bytes()).expect("headers should be valid");
    let body = tokio_util::bytes::Bytes::from_static(body);

    Message::new(start_line, headers, body, parsing_mode)
}

fn message(start_line: &[u8], header_lines: &str, body: &'static [u8]) -> Message {
    parse(start_line, header_lines, body, ParsingMode::Lenient).expect("message should be valid")
}

fn message_err(start_line: &[u8]) -> MessageError {
    parse(start_line, "", b"", ParsingMode::Lenient).expect_err("message should be invalid")
}

#[test]
fn new_returns_request_for_request_line() {
    let message = message(b"OPTIONS * RTSP/1.0", "CSeq: 1", b"");

    assert!(matches!(message, Message::Request(_)), "unexpected message: {message:?}");
}

#[test]
fn new_returns_response_for_status_line() {
    let message = message(b"RTSP/1.0 200 OK", "CSeq: 1", b"");

    assert!(matches!(message, Message::Response(_)), "unexpected message: {message:?}");
}

#[test]
fn new_rejects_bare_version_prefix() {
    let err = message_err(b"RTSP/");

    assert!(
        matches!(&err, MessageError::InvalidStatusLine(line) if line == "RTSP/"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_rejects_empty_start_line() {
    let err = message_err(b"");

    assert!(
        matches!(&err, MessageError::InvalidRequestLine(line) if line.is_empty()),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_returns_response_for_lowercase_version() {
    let message = message(b"rtsp/1.0 200 OK", "", b"");

    assert!(matches!(message, Message::Response(_)), "unexpected message: {message:?}");
}

#[test]
fn new_returns_response_for_status_line_with_leading_whitespace() {
    let message = message(b" \t RTSP/1.0 200 OK", "", b"");

    assert!(matches!(message, Message::Response(_)), "unexpected message: {message:?}");
}

#[test]
fn new_returns_request_for_request_line_with_leading_whitespace() {
    let message = message(b"  OPTIONS * RTSP/1.0", "", b"");

    assert!(matches!(message, Message::Request(_)), "unexpected message: {message:?}");
}

#[test]
fn new_returns_response_for_http_status_line() {
    let message = message(b"HTTP/1.1 200 OK", "", b"");

    assert!(matches!(message, Message::Response(_)), "unexpected message: {message:?}");
}

#[test]
fn new_returns_request_when_only_uri_contains_slash() {
    let message = message(b"DESCRIBE rtsp://example.com/stream RTSP/1.0", "", b"");

    assert!(matches!(message, Message::Request(_)), "unexpected message: {message:?}");
}

#[test]
fn new_rejects_start_line_with_invalid_utf8() {
    let err = message_err(b"OPTIONS \xff RTSP/1.0");

    assert!(matches!(err, MessageError::InvalidEncoding(_)), "unexpected error: {err:?}");
}

#[test]
fn new_passes_parsing_mode_to_request_parsing() {
    let err = parse(b"OPTIONS * RTSP/1.0", "", b"", ParsingMode::Strict)
        .expect_err("message should be invalid");

    assert!(
        matches!(&err, MessageError::MissingHeader(name) if name == "CSeq"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_passes_parsing_mode_to_response_parsing() {
    let err = parse(b"HTTP/1.1 200 OK", "", b"", ParsingMode::Strict)
        .expect_err("message should be invalid");

    assert!(
        matches!(&err, MessageError::InvalidStatusLine(line) if line == "HTTP/1.1 200 OK"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn display_prints_request_as_received() {
    let message = message(
        b"DESCRIBE rtsp://192.168.1.10:554/stream1 RTSP/1.0",
        "CSeq: 2\r\nAccept: application/sdp\r\nUser-Agent: MyClient/1.0",
        b"",
    );

    assert_eq!(
        message.to_string(),
        "DESCRIBE rtsp://192.168.1.10:554/stream1 RTSP/1.0\n\
         CSeq: 2\n\
         Accept: application/sdp\n\
         User-Agent: MyClient/1.0"
    );
}

#[test]
fn display_prints_request_body_after_empty_line() {
    let message = message(
        b"ANNOUNCE rtsp://example.com/stream RTSP/1.0",
        "CSeq: 1\r\nContent-Length: 13",
        b"v=0\r\ns=Test\r\n",
    );

    assert_eq!(
        message.to_string(),
        "ANNOUNCE rtsp://example.com/stream RTSP/1.0\n\
         CSeq: 1\n\
         Content-Length: 13\n\
         \n\
         v=0\n\
         s=Test"
    );
}

#[test]
fn display_prints_response_as_received() {
    let message = message(b"RTSP/1.0 200 OK", "CSeq: 2\r\nContent-Length: 5", b"hello");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nCSeq: 2\nContent-Length: 5\n\nhello");
}

#[test]
fn display_prints_reason_phrase_with_spaces() {
    let message = message(b"RTSP/1.0 454 Session Not Found", "CSeq: 4", b"");

    assert_eq!(message.to_string(), "RTSP/1.0 454 Session Not Found\nCSeq: 4");
}

#[test]
fn display_omits_missing_reason_phrase() {
    let message = message(b"RTSP/1.0 200", "", b"");

    assert_eq!(message.to_string(), "RTSP/1.0 200");
}

#[test]
fn display_without_headers_or_body() {
    let message = message(b"OPTIONS * RTSP/1.0", "", b"");

    assert_eq!(message.to_string(), "OPTIONS * RTSP/1.0");
}

#[test]
fn display_prints_body_without_headers() {
    let message = message(b"RTSP/1.0 200 OK", "", "héllo".as_bytes());

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\n\nhéllo");
}

#[test]
fn display_prints_extension_method_and_other_version() {
    let message = message(b"GET /stream HTTP/1.1", "", b"");

    assert_eq!(message.to_string(), "GET /stream HTTP/1.1");
}

#[test]
fn display_prints_normalized_start_line() {
    let message = message(b"  options   *  rtsp/1.0 ", "", b"");

    assert_eq!(message.to_string(), "OPTIONS * RTSP/1.0");
}

#[test]
fn display_replaces_invalid_utf8_in_body() {
    let message = message(b"RTSP/1.0 200 OK", "", b"a\xffb");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\n\na\u{fffd}b");
}

#[test]
fn display_escapes_control_characters_in_start_line() {
    let message = message(b"X\x1b[31m * RTSP/1.0", "", b"");

    assert_eq!(message.to_string(), r"X\u{1b}[31m * RTSP/1.0");
}

#[test]
fn display_escapes_control_characters_in_headers() {
    let message = message(b"OPTIONS * RTSP/1.0", "X-Test: a\x1b[31mb\nc", b"");

    assert_eq!(message.to_string(), concat!("OPTIONS * RTSP/1.0\n", r"X-Test: a\u{1b}[31mb\nc"));
}

#[test]
fn display_escapes_control_characters_in_body() {
    let message = message(b"RTSP/1.0 200 OK", "", b"a\x1b[31mb\r\nc");

    assert_eq!(message.to_string(), concat!("RTSP/1.0 200 OK\n\n", r"a\u{1b}[31mb", "\nc"));
}

fn encode(message: &Message) -> String {
    let mut dst = tokio_util::bytes::BytesMut::new();
    message.encode(&mut dst);

    String::from_utf8(dst.to_vec()).expect("encoded message should be UTF-8")
}

#[test]
fn encode_writes_request_with_full_uri() {
    let message = message(
        b"DESCRIBE rtsp://192.168.1.10:554/stream1 RTSP/1.0",
        "CSeq: 2\r\nAccept: application/sdp\r\nUser-Agent: MyClient/1.0",
        b"",
    );

    assert_eq!(
        encode(&message),
        "DESCRIBE rtsp://192.168.1.10:554/stream1 RTSP/1.0\r\n\
         CSeq: 2\r\n\
         Accept: application/sdp\r\n\
         User-Agent: MyClient/1.0\r\n\
         \r\n"
    );
}

#[test]
fn encode_ends_message_head_with_empty_line() {
    let message = message(b"OPTIONS * RTSP/1.0", "", b"");

    assert_eq!(encode(&message), "OPTIONS * RTSP/1.0\r\n\r\n");
}

#[test]
fn encode_keeps_query_of_uri() {
    let message = message(b"PLAY rtsp://example.com/stream?track=1 RTSP/1.0", "CSeq: 3", b"");

    assert_eq!(
        encode(&message),
        "PLAY rtsp://example.com/stream?track=1 RTSP/1.0\r\nCSeq: 3\r\n\r\n"
    );
}

#[test]
fn encode_writes_request_body_after_message_head() {
    let message = message(
        b"ANNOUNCE rtsp://example.com/stream RTSP/1.0",
        "CSeq: 1\r\nContent-Type: application/sdp",
        b"v=0\r\ns=Test\r\n",
    );

    assert_eq!(
        encode(&message),
        "ANNOUNCE rtsp://example.com/stream RTSP/1.0\r\n\
         CSeq: 1\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: 13\r\n\
         \r\n\
         v=0\r\n\
         s=Test\r\n"
    );
}

#[test]
fn encode_writes_response_with_body() {
    let message = message(b"RTSP/1.0 200 OK", "CSeq: 2\r\nContent-Length: 5", b"hello");

    assert_eq!(encode(&message), "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nhello");
}

#[test]
fn encode_writes_reason_phrase_with_spaces() {
    let message = message(b"RTSP/1.0 454 Session Not Found", "CSeq: 4", b"");

    assert_eq!(encode(&message), "RTSP/1.0 454 Session Not Found\r\nCSeq: 4\r\n\r\n");
}

#[test]
fn encode_keeps_space_before_missing_reason_phrase() {
    // The status line grammar requires the space even when the reason phrase is empty.
    let message = message(b"RTSP/1.0 200", "", b"");

    assert_eq!(encode(&message), "RTSP/1.0 200 \r\n\r\n");
}

#[test]
fn encode_writes_extension_method_and_other_version() {
    let message = message(b"GET /stream HTTP/1.1", "", b"");

    assert_eq!(encode(&message), "GET /stream HTTP/1.1\r\n\r\n");
}

#[test]
fn encode_writes_normalized_start_line() {
    let message = message(b"  options   *  rtsp/1.0 ", "", b"");

    assert_eq!(encode(&message), "OPTIONS * RTSP/1.0\r\n\r\n");
}

#[test]
fn encode_corrects_content_length_to_body_length() {
    let message = message(b"RTSP/1.0 200 OK", "Content-Length: 99", b"hi");

    assert_eq!(encode(&message), "RTSP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nhi");
}

#[test]
fn encode_keeps_binary_body_unchanged() {
    let message = message(b"RTSP/1.0 200 OK", "", b"\x00\xff\r\n");
    let mut dst = tokio_util::bytes::BytesMut::new();

    message.encode(&mut dst);

    assert_eq!(&dst[..], b"RTSP/1.0 200 OK\r\nContent-Length: 4\r\n\r\n\x00\xff\r\n");
}
