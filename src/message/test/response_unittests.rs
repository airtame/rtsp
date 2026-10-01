use super::*;

fn response(start_line: &[u8]) -> Response {
    Response::parse(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
        .expect("status line should be valid")
}

fn response_err(start_line: &[u8]) -> MessageError {
    Response::parse(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
        .expect_err("status line should be invalid")
}

fn assert_invalid_status_line(start_line: &str) {
    let err = response_err(start_line.as_bytes());

    assert!(
        matches!(&err, MessageError::InvalidStatusLine(line) if line == start_line),
        "unexpected error: {err:?}"
    );
}

#[test]
fn parse_reads_status_line() {
    let response = response(b"RTSP/1.0 200 OK");

    assert_eq!(response.version, Version::V1);
    assert_eq!(response.status_code, 200);
    assert_eq!(response.reason_phrase, "OK");
}

#[test]
fn parse_keeps_spaces_inside_reason_phrase() {
    let response = response(b"RTSP/2.0 454 Session Not Found");

    assert_eq!(response.version, Version::V2);
    assert_eq!(response.status_code, 454);
    assert_eq!(response.reason_phrase, "Session Not Found");
}

#[test]
fn parse_accepts_missing_reason_phrase() {
    let response = response(b"RTSP/1.0 200");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.reason_phrase, "");
}

#[test]
fn parse_accepts_extra_whitespace_between_parts() {
    let response = response(b"  RTSP/1.0   404   Not Found  ");

    assert_eq!(response.version, Version::V1);
    assert_eq!(response.status_code, 404);
    assert_eq!(response.reason_phrase, "Not Found");
}

#[test]
fn parse_accepts_lowercase_version() {
    let response = response(b"rtsp/1.0 200 OK");

    assert_eq!(response.version, Version::V1);
}

#[test]
fn parse_reads_other_protocol_version() {
    let response = response(b"HTTP/1.1 200 OK");

    assert_eq!(response.version, Version::Other("HTTP/1.1".to_owned()));
}

#[test]
fn parse_keeps_headers_and_body() {
    let headers = MessageHeaders::try_from(b"CSeq: 2".as_slice()).expect("headers should be valid");
    let body = tokio_util::bytes::Bytes::from_static(b"hello");

    let response =
        Response::parse(b"RTSP/1.0 200 OK", headers, body).expect("status line should be valid");

    assert_eq!(response.headers.get("CSeq"), Some("2"));
    assert_eq!(&response.body[..], b"hello");
}

#[test]
fn parse_rejects_empty_status_line() {
    assert_invalid_status_line("");
}

#[test]
fn parse_rejects_version_only() {
    assert_invalid_status_line("RTSP/1.0");
}

#[test]
fn parse_rejects_unknown_rtsp_version() {
    assert_invalid_status_line("RTSP/1.1 200 OK");
}

#[test]
fn parse_rejects_non_numeric_status_code() {
    assert_invalid_status_line("RTSP/1.0 abc OK");
}

#[test]
fn parse_rejects_status_code_with_wrong_length() {
    assert_invalid_status_line("RTSP/1.0 20 OK");
    assert_invalid_status_line("RTSP/1.0 2000 OK");
}

#[test]
fn parse_rejects_signed_status_code() {
    assert_invalid_status_line("RTSP/1.0 +20 OK");
}

#[test]
fn parse_rejects_invalid_utf8() {
    let err = response_err(b"RTSP/1.0 200 \xff");

    assert!(
        matches!(&err, MessageError::InvalidEncoding(text) if text == "RTSP/1.0 200 \u{fffd}"),
        "unexpected error: {err:?}"
    );
}

fn encode(response: &Response) -> String {
    let mut dst = tokio_util::bytes::BytesMut::new();
    response.encode(&mut dst);

    String::from_utf8(dst.to_vec()).expect("encoded response should be UTF-8")
}

#[test]
fn new_sets_version_and_status_code() {
    let response = Response::new(Version::V2, StatusCode::NotFound);

    assert_eq!(response.version(), &Version::V2);
    assert_eq!(response.status_code(), 404);
}

#[test]
fn new_uses_reason_phrase_of_status_code() {
    let response = Response::new(Version::V1, StatusCode::SessionNotFound);

    assert_eq!(response.reason_phrase(), "Session Not Found");
}

#[test]
fn new_has_no_headers_or_body() {
    let response = Response::new(Version::V1, StatusCode::Ok);

    assert!(response.headers().is_empty());
    assert!(response.body().is_empty());
}

#[test]
fn with_reason_phrase_replaces_standard_reason_phrase() {
    let response = Response::new(Version::V1, StatusCode::Ok).with_reason_phrase("Everything Fine");

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.reason_phrase(), "Everything Fine");
}

#[test]
fn with_header_adds_headers_in_order() {
    let response = Response::new(Version::V1, StatusCode::Ok)
        .with_header("CSeq", "2")
        .with_header("Session", String::from("12345678"));

    assert_eq!(response.headers().get("CSeq"), Some("2"));
    assert_eq!(response.headers().get("Session"), Some("12345678"));
    assert_eq!(response.headers().to_string(), "CSeq: 2\nSession: 12345678");
}

#[test]
fn with_header_keeps_headers_with_the_same_name() {
    let response = Response::new(Version::V1, StatusCode::Ok)
        .with_header("Public", "OPTIONS")
        .with_header("Public", "DESCRIBE");

    assert_eq!(response.headers().to_string(), "Public: OPTIONS\nPublic: DESCRIBE");
}

#[test]
#[should_panic(expected = "invalid header value")]
fn with_header_rejects_line_break_in_value() {
    let _ = Response::new(Version::V1, StatusCode::Ok).with_header("CSeq", "1\r\nX-Injected: yes");
}

#[test]
#[should_panic(expected = "invalid header value")]
fn with_header_rejects_bare_newline_in_value() {
    let _ = Response::new(Version::V1, StatusCode::Ok).with_header("CSeq", "1\nX-Injected: yes");
}

#[test]
#[should_panic(expected = "invalid header name")]
fn with_header_rejects_empty_name() {
    let _ = Response::new(Version::V1, StatusCode::Ok).with_header("", "1");
}

#[test]
#[should_panic(expected = "invalid header name")]
fn with_header_rejects_colon_in_name() {
    let _ = Response::new(Version::V1, StatusCode::Ok).with_header("CSeq: 1\r\nX-Injected", "yes");
}

#[test]
fn with_body_accepts_text_and_bytes() {
    let from_str = Response::new(Version::V1, StatusCode::Ok).with_body("v=0\r\n");
    let from_string = Response::new(Version::V1, StatusCode::Ok).with_body(String::from("v=0\r\n"));
    let from_vec = Response::new(Version::V1, StatusCode::Ok).with_body(b"v=0\r\n".to_vec());
    let from_slice = Response::new(Version::V1, StatusCode::Ok).with_body(&b"v=0\r\n"[..]);

    for response in [from_str, from_string, from_vec, from_slice] {
        assert_eq!(response.body(), b"v=0\r\n");
    }
}

#[test]
fn with_body_replaces_previous_body() {
    let response =
        Response::new(Version::V1, StatusCode::Ok).with_body("first").with_body("second");

    assert_eq!(response.body(), b"second");
}

#[test]
fn new_response_encodes_to_status_line_and_empty_head() {
    let response = Response::new(Version::V1, StatusCode::Ok);

    assert_eq!(encode(&response), "RTSP/1.0 200 OK\r\n\r\n");
}

#[test]
fn created_response_encodes_headers_body_and_content_length() {
    let response = Response::new(Version::V1, StatusCode::Ok)
        .with_header("CSeq", "2")
        .with_header("Content-Type", "application/sdp")
        .with_body("v=0\r\ns=Test\r\n");

    assert_eq!(
        encode(&response),
        "RTSP/1.0 200 OK\r\n\
         CSeq: 2\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: 13\r\n\
         \r\n\
         v=0\r\n\
         s=Test\r\n"
    );
}

#[test]
fn created_response_displays_headers_and_body() {
    let response = Response::new(Version::V2, StatusCode::SessionNotFound)
        .with_header("CSeq", "4")
        .with_body("gone");

    assert_eq!(response.to_string(), "RTSP/2.0 454 Session Not Found\nCSeq: 4\n\ngone");
}

#[test]
fn created_response_parses_back_to_same_fields() {
    let created = Response::new(Version::V2, StatusCode::ServiceUnavailable)
        .with_reason_phrase("Try Later")
        .with_header("CSeq", "5")
        .with_body("retry");
    let encoded = encode(&created);
    let (head, body) = encoded.split_once("\r\n\r\n").expect("encoded response has a head");
    let (status_line, header_lines) =
        head.split_once("\r\n").expect("encoded response has headers");

    let headers =
        MessageHeaders::try_from(header_lines.as_bytes()).expect("encoded headers should parse");
    let parsed = Response::parse(
        status_line.as_bytes(),
        headers,
        tokio_util::bytes::Bytes::copy_from_slice(body.as_bytes()),
    )
    .expect("encoded status line should parse");

    assert_eq!(parsed.version(), created.version());
    assert_eq!(parsed.status_code(), created.status_code());
    assert_eq!(parsed.reason_phrase(), created.reason_phrase());
    assert_eq!(parsed.headers().get("CSeq"), Some("5"));
    assert_eq!(parsed.headers().get("Content-Length"), Some("5"));
    assert_eq!(parsed.body(), created.body());
}
