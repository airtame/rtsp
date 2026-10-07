use super::*;
use crate::message::CSeqHeader;

fn parse(lines: &str) -> MessageHeaders {
    MessageHeaders::try_from(lines.as_bytes()).expect("headers should be valid")
}

#[test]
fn try_from_parses_name_value_pairs() {
    let headers = parse("CSeq: 1\r\nSession: 12345678");

    assert_eq!(headers.get("CSeq"), Some("1"));
    assert_eq!(headers.get("Session"), Some("12345678"));
}

#[test]
fn get_accepts_header_name() {
    let headers = parse("CSeq: 1\r\nX-Custom: a");

    assert_eq!(headers.get(MessageHeaderName::CSeq), Some("1"));
    assert_eq!(headers.get(MessageHeaderName::Extension("x-custom".to_owned())), Some("a"));
}

#[test]
fn get_ignores_case_of_name() {
    let headers = parse("cseq: 1");

    assert_eq!(headers.get("CSEQ"), Some("1"));
}

#[test]
fn get_returns_first_of_repeated_values() {
    let headers = parse("Public: OPTIONS\r\nPublic: DESCRIBE");

    assert_eq!(headers.get(MessageHeaderName::Public), Some("OPTIONS"));
}

#[test]
fn get_all_returns_every_value_in_order() {
    let headers = parse("Public: OPTIONS\r\nCSeq: 1\r\npublic: DESCRIBE");

    let values = headers.get_all(MessageHeaderName::Public).collect::<Vec<_>>();

    assert_eq!(values, ["OPTIONS", "DESCRIBE"]);
}

#[test]
fn get_all_returns_nothing_for_missing_header() {
    let headers = parse("CSeq: 1");

    assert_eq!(headers.get_all("Public").count(), 0);
}

#[test]
fn typed_decodes_header() {
    let headers = parse("CSeq: 7");

    let cseq = headers.typed::<CSeqHeader>().expect("CSeq should be present");

    assert_eq!(cseq.expect("CSeq should be valid"), CSeqHeader(7));
}

#[test]
fn typed_returns_none_for_missing_header() {
    let headers = parse("Session: 12345678");

    assert!(headers.typed::<CSeqHeader>().is_none());
}

#[test]
fn typed_returns_error_for_invalid_value() {
    let headers = parse("CSeq: abc");

    let cseq = headers.typed::<CSeqHeader>().expect("CSeq should be present");

    assert!(
        matches!(&cseq, Err(MessageError::InvalidHeaderValue(MessageHeaderName::CSeq, value)) if value == "abc"),
        "unexpected result: {cseq:?}"
    );
}

#[test]
fn iter_returns_every_header_in_order() {
    let headers = parse("CSeq: 1\r\nX-Custom: a\r\npublic: OPTIONS");

    let fields = headers.iter().collect::<Vec<_>>();

    assert_eq!(
        fields,
        [
            (&MessageHeaderName::CSeq, "1"),
            (&MessageHeaderName::Extension("X-Custom".to_owned()), "a"),
            (&MessageHeaderName::Public, "OPTIONS")
        ]
    );
}

#[test]
fn try_from_trims_whitespace_around_name_and_value() {
    let headers = parse("  CSeq  :   1  ");

    assert_eq!(headers.get("CSeq"), Some("1"));
}

#[test]
fn try_from_splits_on_first_colon_only() {
    let headers = parse("Content-Base: rtsp://example.com:554/stream/");

    assert_eq!(headers.get("Content-Base"), Some("rtsp://example.com:554/stream/"));
}

#[test]
fn try_from_accepts_empty_value() {
    let headers = parse("Require:");

    assert_eq!(headers.get("Require"), Some(""));
}

#[test]
fn try_from_returns_no_headers_for_empty_input() {
    let headers = parse("");

    assert!(headers.fields.is_empty());
}

#[test]
fn try_from_ignores_trailing_line_terminator() {
    let headers = parse("CSeq: 1\r\n");

    assert_eq!(headers.fields, [(MessageHeaderName::CSeq, "1".to_owned())]);
}

#[test]
fn try_from_keeps_repeated_headers_in_order() {
    let headers = parse("Public: OPTIONS\r\nPublic: DESCRIBE");

    assert_eq!(
        headers.fields,
        [
            (MessageHeaderName::Public, "OPTIONS".to_owned()),
            (MessageHeaderName::Public, "DESCRIBE".to_owned())
        ]
    );
}

#[test]
fn try_from_rejects_line_without_colon() {
    let err = MessageHeaders::try_from("CSeq: 1\r\nSession 12345678".as_bytes()).unwrap_err();

    assert!(
        matches!(&err, MessageError::InvalidHeader(line) if line == "Session 12345678"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn try_from_rejects_empty_header_name() {
    let err = MessageHeaders::try_from(" : 1".as_bytes()).unwrap_err();

    assert!(
        matches!(&err, MessageError::InvalidHeader(line) if line == " : 1"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn try_from_rejects_invalid_utf8() {
    let err = MessageHeaders::try_from(b"CSeq: \xff".as_slice()).unwrap_err();

    assert!(
        matches!(&err, MessageError::InvalidEncoding(text) if text == "CSeq: \u{fffd}"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn get_is_case_insensitive() {
    let headers = parse("CSeq: 1");

    assert_eq!(headers.get("cseq"), Some("1"));
    assert_eq!(headers.get("CSEQ"), Some("1"));
}

#[test]
fn get_returns_first_of_repeated_headers() {
    let headers = parse("Public: OPTIONS\r\nPublic: DESCRIBE");

    assert_eq!(headers.get("Public"), Some("OPTIONS"));
}

#[test]
fn get_returns_none_for_missing_header() {
    let headers = parse("CSeq: 1");

    assert_eq!(headers.get("Session"), None);
}

#[test]
fn default_has_no_headers() {
    assert_eq!(MessageHeaders::default().get("CSeq"), None);
}

#[test]
fn is_empty_without_headers() {
    assert!(MessageHeaders::default().is_empty());
    assert!(parse("").is_empty());
}

#[test]
fn is_not_empty_with_headers() {
    assert!(!parse("CSeq: 1").is_empty());
}

#[test]
fn display_prints_one_header_per_line() {
    let headers = parse("CSeq: 1\r\nSession: 12345678");

    assert_eq!(headers.to_string(), "CSeq: 1\nSession: 12345678");
}

#[test]
fn display_prints_nothing_without_headers() {
    assert_eq!(MessageHeaders::default().to_string(), "");
}

fn encode(headers: &MessageHeaders, body_length: usize) -> String {
    let mut dst = tokio_util::bytes::BytesMut::new();
    headers.encode(body_length, &mut dst);

    String::from_utf8(dst.to_vec()).expect("encoded headers should be UTF-8")
}

#[test]
fn encode_writes_each_header_on_its_own_line() {
    let headers = parse("CSeq: 1\r\nSession: 12345678");

    assert_eq!(encode(&headers, 0), "CSeq: 1\r\nSession: 12345678\r\n");
}

#[test]
fn encode_writes_nothing_without_headers_or_body() {
    assert_eq!(encode(&MessageHeaders::default(), 0), "");
}

#[test]
fn encode_keeps_repeated_headers_in_order() {
    let headers = parse("Public: OPTIONS\r\nPublic: DESCRIBE");

    assert_eq!(encode(&headers, 0), "Public: OPTIONS\r\nPublic: DESCRIBE\r\n");
}

#[test]
fn encode_writes_trimmed_names_and_values() {
    let headers = parse("  CSeq  :   1  ");

    assert_eq!(encode(&headers, 0), "CSeq: 1\r\n");
}

#[test]
fn encode_adds_content_length_for_body() {
    let headers = parse("CSeq: 1");

    assert_eq!(encode(&headers, 5), "CSeq: 1\r\nContent-Length: 5\r\n");
}

#[test]
fn encode_adds_content_length_without_other_headers() {
    assert_eq!(encode(&MessageHeaders::default(), 5), "Content-Length: 5\r\n");
}

#[test]
fn encode_replaces_content_length_with_body_length() {
    let headers = parse("Content-Length: 99\r\nCSeq: 1");

    assert_eq!(encode(&headers, 2), "Content-Length: 2\r\nCSeq: 1\r\n");
}

#[test]
fn encode_keeps_position_of_content_length_and_writes_its_standard_name() {
    let headers = parse("CSeq: 1\r\ncontent-length: 5\r\nSession: 12345678");

    assert_eq!(encode(&headers, 5), "CSeq: 1\r\nContent-Length: 5\r\nSession: 12345678\r\n");
}

#[test]
fn encode_writes_standard_spelling_of_known_names() {
    let headers = parse("cseq: 1\r\nWWW-AUTHENTICATE: Basic");

    assert_eq!(encode(&headers, 0), "CSeq: 1\r\nWWW-Authenticate: Basic\r\n");
}

#[test]
fn encode_keeps_spelling_of_extension_names() {
    let headers = parse("x-Custom: 1");

    assert_eq!(encode(&headers, 0), "x-Custom: 1\r\n");
}

#[test]
fn encode_drops_duplicate_content_length() {
    let headers = parse("Content-Length: 5\r\nCSeq: 1\r\nContent-Length: 7");

    assert_eq!(encode(&headers, 5), "Content-Length: 5\r\nCSeq: 1\r\n");
}

#[test]
fn encode_keeps_zero_content_length_header() {
    let headers = parse("Content-Length: 0");

    assert_eq!(encode(&headers, 0), "Content-Length: 0\r\n");
}

#[test]
fn encode_appends_to_existing_buffer() {
    let headers = parse("CSeq: 1");
    let mut dst = tokio_util::bytes::BytesMut::from(&b"existing"[..]);

    headers.encode(0, &mut dst);

    assert_eq!(&dst[..], b"existingCSeq: 1\r\n");
}
