use super::*;

fn request(start_line: &[u8]) -> Request {
    Request::new(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
        .expect("request line should be valid")
}

fn request_err(start_line: &[u8]) -> MessageError {
    Request::new(start_line, MessageHeaders::default(), tokio_util::bytes::Bytes::new())
        .expect_err("request line should be invalid")
}

fn assert_invalid_request_line(start_line: &str) {
    let err = request_err(start_line.as_bytes());

    assert!(
        matches!(&err, MessageError::InvalidRequestLine(line) if line == start_line),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_parses_request_line() {
    let request = request(b"DESCRIBE rtsp://example.com/stream RTSP/1.0");

    assert_eq!(request.method, RequestMethod::Describe);
    assert_eq!(request.uri, "rtsp://example.com/stream");
    assert_eq!(request.path, "/stream");
    assert_eq!(request.query, None);
    assert_eq!(request.version, Version::V1);
}

#[test]
fn new_parses_query() {
    let request = request(b"PLAY rtsp://example.com/stream?track=1 RTSP/2.0");

    assert_eq!(request.method, RequestMethod::Play);
    assert_eq!(request.uri, "rtsp://example.com/stream?track=1");
    assert_eq!(request.path, "/stream");
    assert_eq!(request.query.as_deref(), Some("track=1"));
    assert_eq!(request.version, Version::V2);
}

#[test]
fn new_parses_asterisk_uri() {
    let request = request(b"OPTIONS * RTSP/1.0");

    assert_eq!(request.method, RequestMethod::Options);
    assert_eq!(request.path, "*");
    assert_eq!(request.query, None);
}

#[test]
fn new_parses_extension_method() {
    let request = request(b"PLAY_NOTIFY rtsp://example.com/stream RTSP/2.0");

    assert_eq!(request.method, RequestMethod::Extension("PLAY_NOTIFY".to_owned()));
}

#[test]
fn new_parses_other_protocol_version() {
    let request = request(b"GET /stream HTTP/1.1");

    assert_eq!(request.method, RequestMethod::Extension("GET".to_owned()));
    assert_eq!(request.path, "/stream");
    assert_eq!(request.version, Version::Other("HTTP/1.1".to_owned()));
}

#[test]
fn new_accepts_lowercase_method_and_version() {
    let request = request(b"setup rtsp://example.com/stream/track1 rtsp/1.0");

    assert_eq!(request.method, RequestMethod::Setup);
    assert_eq!(request.path, "/stream/track1");
    assert_eq!(request.version, Version::V1);
}

#[test]
fn new_accepts_extra_whitespace_between_parts() {
    let request = request(b"  TEARDOWN \t rtsp://example.com/stream   RTSP/1.0  ");

    assert_eq!(request.method, RequestMethod::Teardown);
    assert_eq!(request.path, "/stream");
    assert_eq!(request.version, Version::V1);
}

#[test]
fn new_keeps_headers_and_body() {
    let headers = MessageHeaders::try_from(b"CSeq: 1".as_slice()).expect("headers should be valid");
    let body = tokio_util::bytes::Bytes::from_static(b"v=0\r\n");

    let request = Request::new(b"ANNOUNCE rtsp://example.com/stream RTSP/1.0", headers, body)
        .expect("request line should be valid");

    assert_eq!(request.headers.get("CSeq"), Some("1"));
    assert_eq!(&request.body[..], b"v=0\r\n");
}

#[test]
fn new_rejects_empty_request_line() {
    assert_invalid_request_line("");
}

#[test]
fn new_rejects_whitespace_only_request_line() {
    assert_invalid_request_line("   ");
}

#[test]
fn new_rejects_missing_version() {
    assert_invalid_request_line("OPTIONS *");
}

#[test]
fn new_rejects_method_only() {
    assert_invalid_request_line("OPTIONS");
}

#[test]
fn new_rejects_extra_parts() {
    assert_invalid_request_line("OPTIONS * RTSP/1.0 extra");
}

#[test]
fn new_rejects_unknown_rtsp_version() {
    assert_invalid_request_line("OPTIONS * RTSP/1.1");
}

#[test]
fn new_rejects_malformed_version() {
    assert_invalid_request_line("OPTIONS * RTSP");
}

#[test]
fn new_rejects_invalid_utf8() {
    let err = request_err(b"OPTIONS \xff RTSP/1.0");

    assert!(
        matches!(&err, MessageError::InvalidEncoding(text) if text == "OPTIONS \u{fffd} RTSP/1.0"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn parse_uri_strips_scheme_and_host() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com:554/live/stream"),
        ("/live/stream".to_owned(), None)
    );
}

#[test]
fn parse_uri_splits_query() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com/stream?track=1&rate=2"),
        ("/stream".to_owned(), Some("track=1&rate=2".to_owned()))
    );
}

#[test]
fn parse_uri_uses_root_path_when_uri_has_no_path() {
    assert_eq!(Request::parse_uri("rtsp://example.com"), ("/".to_owned(), None));
    assert_eq!(Request::parse_uri("rtsp://example.com:554"), ("/".to_owned(), None));
}

#[test]
fn parse_uri_keeps_query_when_uri_has_no_path() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com?track=1"),
        ("/".to_owned(), Some("track=1".to_owned()))
    );
}

#[test]
fn parse_uri_keeps_empty_query() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com/stream?"),
        ("/stream".to_owned(), Some(String::new()))
    );
}

#[test]
fn parse_uri_splits_query_on_first_question_mark() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com/stream?a=1?b=2"),
        ("/stream".to_owned(), Some("a=1?b=2".to_owned()))
    );
}

#[test]
fn parse_uri_keeps_path_without_scheme() {
    assert_eq!(
        Request::parse_uri("/stream?track=1"),
        ("/stream".to_owned(), Some("track=1".to_owned()))
    );
}

#[test]
fn parse_uri_keeps_asterisk() {
    assert_eq!(Request::parse_uri("*"), ("*".to_owned(), None));
}

#[test]
fn parse_uri_handles_ipv6_host() {
    assert_eq!(Request::parse_uri("rtsp://[::1]:8554/stream"), ("/stream".to_owned(), None));
}

#[test]
fn parse_uri_strips_fragment() {
    assert_eq!(Request::parse_uri("rtsp://example.com/stream#t=10"), ("/stream".to_owned(), None));
}

#[test]
fn parse_uri_strips_fragment_after_query() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com/stream?track=1#t=10"),
        ("/stream".to_owned(), Some("track=1".to_owned()))
    );
}

#[test]
fn parse_uri_strips_fragment_when_uri_has_no_path() {
    assert_eq!(Request::parse_uri("rtsp://example.com#t=10"), ("/".to_owned(), None));
}

#[test]
fn parse_uri_strips_fragment_without_scheme() {
    assert_eq!(Request::parse_uri("/stream#t=10"), ("/stream".to_owned(), None));
}

#[test]
fn parse_uri_keeps_scheme_separator_inside_query() {
    assert_eq!(
        Request::parse_uri("/stream?redirect=rtsp://other.com/live"),
        ("/stream".to_owned(), Some("redirect=rtsp://other.com/live".to_owned()))
    );
}

#[test]
fn parse_uri_keeps_scheme_separator_inside_query_of_absolute_uri() {
    assert_eq!(
        Request::parse_uri("rtsp://example.com/stream?redirect=rtsp://other.com/live"),
        ("/stream".to_owned(), Some("redirect=rtsp://other.com/live".to_owned()))
    );
}

#[test]
fn parse_uri_keeps_scheme_separator_inside_path() {
    assert_eq!(
        Request::parse_uri("/proxy/rtsp://other.com/live"),
        ("/proxy/rtsp://other.com/live".to_owned(), None)
    );
}

#[test]
fn parse_uri_strips_user_info() {
    assert_eq!(
        Request::parse_uri("rtsp://user:secret@example.com/stream"),
        ("/stream".to_owned(), None)
    );
}
