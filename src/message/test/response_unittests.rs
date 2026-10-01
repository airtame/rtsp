use super::*;

fn response(start_line: &[u8]) -> Response {
    Response::new(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
        .expect("status line should be valid")
}

fn response_err(start_line: &[u8]) -> MessageError {
    Response::new(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
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
fn new_parses_status_line() {
    let response = response(b"RTSP/1.0 200 OK");

    assert_eq!(response.version, Version::V1);
    assert_eq!(response.status_code, 200);
    assert_eq!(response.reason_phrase, "OK");
}

#[test]
fn new_keeps_spaces_inside_reason_phrase() {
    let response = response(b"RTSP/2.0 454 Session Not Found");

    assert_eq!(response.version, Version::V2);
    assert_eq!(response.status_code, 454);
    assert_eq!(response.reason_phrase, "Session Not Found");
}

#[test]
fn new_accepts_missing_reason_phrase() {
    let response = response(b"RTSP/1.0 200");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.reason_phrase, "");
}

#[test]
fn new_accepts_extra_whitespace_between_parts() {
    let response = response(b"  RTSP/1.0   404   Not Found  ");

    assert_eq!(response.version, Version::V1);
    assert_eq!(response.status_code, 404);
    assert_eq!(response.reason_phrase, "Not Found");
}

#[test]
fn new_accepts_lowercase_version() {
    let response = response(b"rtsp/1.0 200 OK");

    assert_eq!(response.version, Version::V1);
}

#[test]
fn new_parses_other_protocol_version() {
    let response = response(b"HTTP/1.1 200 OK");

    assert_eq!(response.version, Version::Other("HTTP/1.1".to_owned()));
}

#[test]
fn new_keeps_headers_and_body() {
    let headers = MessageHeaders::try_from(b"CSeq: 2".as_slice()).expect("headers should be valid");
    let body = tokio_util::bytes::Bytes::from_static(b"hello");

    let response =
        Response::new(b"RTSP/1.0 200 OK", headers, body).expect("status line should be valid");

    assert_eq!(response.headers.get("CSeq"), Some("2"));
    assert_eq!(&response.body[..], b"hello");
}

#[test]
fn new_rejects_empty_status_line() {
    assert_invalid_status_line("");
}

#[test]
fn new_rejects_version_only() {
    assert_invalid_status_line("RTSP/1.0");
}

#[test]
fn new_rejects_unknown_rtsp_version() {
    assert_invalid_status_line("RTSP/1.1 200 OK");
}

#[test]
fn new_rejects_non_numeric_status_code() {
    assert_invalid_status_line("RTSP/1.0 abc OK");
}

#[test]
fn new_rejects_status_code_with_wrong_length() {
    assert_invalid_status_line("RTSP/1.0 20 OK");
    assert_invalid_status_line("RTSP/1.0 2000 OK");
}

#[test]
fn new_rejects_signed_status_code() {
    assert_invalid_status_line("RTSP/1.0 +20 OK");
}

#[test]
fn new_rejects_invalid_utf8() {
    let err = response_err(b"RTSP/1.0 200 \xff");

    assert!(
        matches!(&err, MessageError::InvalidEncoding(text) if text == "RTSP/1.0 200 \u{fffd}"),
        "unexpected error: {err:?}"
    );
}
