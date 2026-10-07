use tokio_util::codec::{Decoder, Encoder};

use super::*;
use crate::message::{Request, RequestMethod, Response, StatusCode, Version};

const OPTIONS_REQUEST: &[u8] = b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n";

fn buffer(bytes: &[u8]) -> tokio_util::bytes::BytesMut {
    tokio_util::bytes::BytesMut::from(bytes)
}

fn decode(src: &mut tokio_util::bytes::BytesMut) -> Option<Message> {
    MessageCodec::default()
        .decode(src)
        .expect("decode should succeed")
        .map(|decoded| decoded.expect("message should be well-formed"))
}

fn decode_err(src: &mut tokio_util::bytes::BytesMut) -> MessageError {
    MessageCodec::default().decode(src).expect_err("decode should fail")
}

fn decode_malformed(src: &mut tokio_util::bytes::BytesMut) -> MalformedMessage {
    MessageCodec::default()
        .decode(src)
        .expect("decode should succeed")
        .expect("message should be complete")
        .expect_err("message should be malformed")
}

fn decode_request(src: &mut tokio_util::bytes::BytesMut) -> Request {
    match decode(src) {
        Some(Message::Request(request)) => request,
        other => panic!("expected a request, got {other:?}"),
    }
}

fn decode_response(src: &mut tokio_util::bytes::BytesMut) -> Response {
    match decode(src) {
        Some(Message::Response(response)) => response,
        other => panic!("expected a response, got {other:?}"),
    }
}

fn decode_lenient_request(src: &mut tokio_util::bytes::BytesMut) -> Request {
    match MessageCodec::new(ParsingMode::Lenient).decode(src).expect("decode should succeed") {
        Some(Ok(Message::Request(request))) => request,
        other => panic!("expected a request, got {other:?}"),
    }
}

#[test]
fn decode_returns_none_for_empty_buffer() {
    let mut src = buffer(b"");

    assert!(decode(&mut src).is_none());
}

#[test]
fn decode_returns_none_until_message_head_is_complete() {
    let mut src = buffer(b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n");

    assert!(decode(&mut src).is_none());
    assert_eq!(&src[..], b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n");
}

#[test]
fn decode_returns_request_and_empties_buffer() {
    let mut src = buffer(OPTIONS_REQUEST);

    let message = decode(&mut src).expect("message should be complete");

    assert!(matches!(message, Message::Request(_)), "unexpected message: {message:?}");
    assert_eq!(message.to_string(), "OPTIONS * RTSP/1.0\nCSeq: 1");
    assert!(src.is_empty());
}

#[test]
fn decode_returns_response_with_body() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nhello");

    let message = decode(&mut src).expect("message should be complete");

    assert!(matches!(message, Message::Response(_)), "unexpected message: {message:?}");
    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nCSeq: 2\nContent-Length: 5\n\nhello");
    assert!(src.is_empty());
}

#[test]
fn decode_returns_message_without_headers() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\n\r\n");

    let message = decode(&mut src).expect("message should be complete");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK");
    assert!(src.is_empty());
}

#[test]
fn decode_waits_until_body_is_complete() {
    let mut codec = MessageCodec::default();
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhel");

    assert!(codec.decode(&mut src).expect("decode should succeed").is_none());

    src.extend_from_slice(b"lo");
    let message = codec.decode(&mut src).expect("decode should succeed");

    assert!(message.is_some());
    assert!(src.is_empty());
}

#[test]
fn decode_reserves_space_for_rest_of_body() {
    let head = b"RTSP/1.0 200 OK\r\nContent-Length: 1000\r\n\r\n";
    let mut src = buffer(head);

    assert!(decode(&mut src).is_none());
    assert!(src.capacity() >= head.len() + 1000, "capacity: {}", src.capacity());
}

#[test]
fn decode_returns_pipelined_messages_one_at_a_time() {
    let mut codec = MessageCodec::default();
    let mut src =
        buffer(b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\nOPTIONS * RTSP/1.0\r\nCSeq: 2\r\n\r\n");

    let first = codec.decode(&mut src).expect("decode should succeed").expect("first message");
    let second = codec.decode(&mut src).expect("decode should succeed").expect("second message");
    let first = first.expect("first message should be well-formed");
    let second = second.expect("second message should be well-formed");

    assert_eq!(first.to_string(), "OPTIONS * RTSP/1.0\nCSeq: 1");
    assert_eq!(second.to_string(), "OPTIONS * RTSP/1.0\nCSeq: 2");
    assert!(src.is_empty());
}

#[test]
fn decode_leaves_start_of_next_message_in_buffer() {
    let mut src = buffer(b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\nDESCRIBE rtsp://exa");

    assert!(decode(&mut src).is_some());
    assert_eq!(&src[..], b"DESCRIBE rtsp://exa");
}

#[test]
fn decode_treats_missing_content_length_as_empty_body() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\nextra");

    let message = decode(&mut src).expect("message should be complete");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nCSeq: 1");
    assert_eq!(&src[..], b"extra");
}

#[test]
fn decode_accepts_zero_content_length() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n");

    let message = decode(&mut src).expect("message should be complete");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nContent-Length: 0");
    assert!(src.is_empty());
}

#[test]
fn decode_reads_content_length_case_insensitively() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\ncontent-length: 5\r\n\r\nhello");

    let message = decode(&mut src).expect("message should be complete");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nContent-Length: 5\n\nhello");
    assert!(src.is_empty());
}

#[test]
fn decode_keeps_blank_lines_inside_body() {
    // The body length comes from Content-Length, so an empty line in an SDP body doesn't end it.
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nContent-Length: 6\r\n\r\na\r\n\r\nb");

    let message = decode(&mut src).expect("message should be complete");

    assert_eq!(message.to_string(), "RTSP/1.0 200 OK\nContent-Length: 6\n\na\n\nb");
    assert!(src.is_empty());
}

#[test]
fn decode_returns_error_for_invalid_content_length() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nContent-Length: abc\r\n\r\n");

    let err = decode_err(&mut src);

    assert!(
        matches!(&err, MessageError::InvalidContentLength(value) if value == "abc"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn decode_returns_error_for_negative_content_length() {
    let mut src = buffer(b"RTSP/1.0 200 OK\r\nContent-Length: -5\r\n\r\n");

    let err = decode_err(&mut src);

    assert!(
        matches!(&err, MessageError::InvalidContentLength(value) if value == "-5"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn decode_returns_error_for_invalid_header() {
    let mut src = buffer(b"OPTIONS * RTSP/1.0\r\nCSeq 1\r\n\r\n");

    let err = decode_err(&mut src);

    assert!(
        matches!(&err, MessageError::InvalidHeader(line) if line == "CSeq 1"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn decode_returns_error_for_header_with_invalid_utf8() {
    let mut src = buffer(b"OPTIONS * RTSP/1.0\r\nCSeq: \xff\r\n\r\n");

    let err = decode_err(&mut src);

    assert!(matches!(err, MessageError::InvalidEncoding(_)), "unexpected error: {err:?}");
}

#[test]
fn decode_parses_whole_request() {
    let mut src = buffer(
        b"PLAY rtsp://example.com/stream?track=1 RTSP/1.0\r\nCSeq: 3\r\nSession: 12345678\r\n\r\n",
    );

    let request = decode_request(&mut src);

    assert_eq!(request.method(), &RequestMethod::Play);
    assert_eq!(request.uri(), "rtsp://example.com/stream?track=1");
    assert_eq!(request.path(), "/stream");
    assert_eq!(request.query(), Some("track=1"));
    assert_eq!(request.version(), &Version::V1);
    assert_eq!(request.headers().get("CSeq"), Some("3"));
    assert_eq!(request.headers().get("Session"), Some("12345678"));
    assert!(request.body().is_empty());
}

#[test]
fn decode_parses_whole_request_with_body() {
    let mut src = buffer(
        b"ANNOUNCE rtsp://example.com/stream RTSP/1.0\r\nCSeq: 1\r\nContent-Length: 13\r\n\r\nv=0\r\ns=Test\r\n",
    );

    let request = decode_request(&mut src);

    assert_eq!(request.method(), &RequestMethod::Announce);
    assert_eq!(request.path(), "/stream");
    assert_eq!(request.query(), None);
    assert_eq!(request.body(), b"v=0\r\ns=Test\r\n");
}

#[test]
fn decode_parses_whole_request_without_headers_in_lenient_mode() {
    let mut src = buffer(b"OPTIONS * RTSP/2.0\r\n\r\n");

    let request = decode_lenient_request(&mut src);

    assert_eq!(request.method(), &RequestMethod::Options);
    assert_eq!(request.path(), "*");
    assert_eq!(request.version(), &Version::V2);
    assert_eq!(request.headers().get("CSeq"), None);
    assert!(src.is_empty());
}

#[test]
fn decode_parses_request_with_other_protocol_version_in_lenient_mode() {
    let mut src = buffer(b"GET /health HTTP/1.1\r\nHost: example.com\r\n\r\n");

    let request = decode_lenient_request(&mut src);

    assert_eq!(request.method(), &RequestMethod::Extension("GET".to_owned()));
    assert_eq!(request.path(), "/health");
    assert_eq!(request.version(), &Version::Other("HTTP/1.1".to_owned()));
    assert!(src.is_empty());
}

#[test]
fn decode_parses_whole_response_with_body() {
    let mut src = buffer(
        b"RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Type: application/sdp\r\nContent-Length: 13\r\n\r\nv=0\r\ns=Test\r\n",
    );

    let response = decode_response(&mut src);

    assert_eq!(response.version(), &Version::V1);
    assert_eq!(response.status_code(), &StatusCode::Ok);
    assert_eq!(response.headers().get("CSeq"), Some("2"));
    assert_eq!(response.headers().get("Content-Type"), Some("application/sdp"));
    assert_eq!(response.body(), b"v=0\r\ns=Test\r\n");
}

#[test]
fn decode_parses_whole_response_with_reason_phrase_containing_spaces() {
    let mut src = buffer(b"RTSP/1.0 299 Partly Done For Now\r\nCSeq: 4\r\n\r\n");

    let response = decode_response(&mut src);

    assert_eq!(
        response.status_code(),
        &StatusCode::Extension(299, "Partly Done For Now".to_owned())
    );
    assert_eq!(response.headers().get("CSeq"), Some("4"));
}

#[test]
fn decode_parses_each_pipelined_message_from_its_own_start_line() {
    let mut src = buffer(
        b"SETUP rtsp://example.com/stream/track1 RTSP/1.0\r\nCSeq: 5\r\n\r\nRTSP/1.0 200 OK\r\nCSeq: 6\r\nContent-Length: 2\r\n\r\nhiTEARDOWN rtsp://example.com/stream RTSP/1.0\r\nCSeq: 7\r\n\r\n",
    );

    let setup = decode_request(&mut src);
    assert_eq!(setup.method(), &RequestMethod::Setup);
    assert_eq!(setup.path(), "/stream/track1");
    assert_eq!(setup.headers().get("CSeq"), Some("5"));

    let response = decode_response(&mut src);
    assert_eq!(response.status_code(), &StatusCode::Ok);
    assert_eq!(response.headers().get("CSeq"), Some("6"));
    assert_eq!(response.body(), b"hi");

    let teardown = decode_request(&mut src);
    assert_eq!(teardown.method(), &RequestMethod::Teardown);
    assert_eq!(teardown.path(), "/stream");
    assert_eq!(teardown.headers().get("CSeq"), Some("7"));

    assert!(src.is_empty());
}

#[test]
fn decode_parses_whole_message_split_across_reads() {
    let mut codec = MessageCodec::default();
    let mut src = buffer(b"DESCRIBE rtsp://example.com/str");

    assert!(codec.decode(&mut src).expect("decode should succeed").is_none());
    src.extend_from_slice(b"eam RTSP/1.0\r\nCSeq: 8\r\nContent-Length: 3\r\n\r\nab");
    assert!(codec.decode(&mut src).expect("decode should succeed").is_none());
    src.extend_from_slice(b"c");

    let request = decode_request(&mut src);

    assert_eq!(request.method(), &RequestMethod::Describe);
    assert_eq!(request.path(), "/stream");
    assert_eq!(request.headers().get("CSeq"), Some("8"));
    assert_eq!(request.body(), b"abc");
}

#[test]
fn decode_returns_malformed_message_for_invalid_request_line() {
    let mut src = buffer(b"OPTIONS\r\nCSeq: 1\r\n\r\n");

    let malformed = decode_malformed(&mut src);

    assert!(
        matches!(&malformed.error, MessageError::InvalidRequestLine(line) if line == "OPTIONS"),
        "unexpected error: {:?}",
        malformed.error
    );
    assert_eq!(malformed.cseq.as_deref(), Some("1"));
    assert!(src.is_empty());
}

#[test]
fn decode_returns_malformed_message_for_invalid_status_line() {
    let mut src = buffer(b"RTSP/1.0 2000 OK\r\nCSeq: 1\r\n\r\n");

    let malformed = decode_malformed(&mut src);

    assert!(
        matches!(&malformed.error, MessageError::InvalidStatusLine(line) if line == "RTSP/1.0 2000 OK"),
        "unexpected error: {:?}",
        malformed.error
    );
    assert_eq!(malformed.cseq.as_deref(), Some("1"));
}

#[test]
fn decode_returns_malformed_message_without_cseq() {
    let mut src = buffer(b"OPTIONS\r\n\r\n");

    let malformed = decode_malformed(&mut src);

    assert_eq!(malformed.cseq, None);
}

#[test]
fn decode_returns_malformed_message_for_request_without_cseq_in_strict_mode() {
    let mut src = buffer(b"OPTIONS * RTSP/1.0\r\n\r\n");

    let malformed = decode_malformed(&mut src);

    assert!(
        matches!(&malformed.error, MessageError::MissingHeader(name) if name == "CSeq"),
        "unexpected error: {:?}",
        malformed.error
    );
    assert_eq!(malformed.cseq, None);
    assert!(src.is_empty());
}

#[test]
fn decode_returns_malformed_message_for_other_protocol_version_in_strict_mode() {
    let mut src = buffer(b"GET /health HTTP/1.1\r\nCSeq: 1\r\n\r\n");

    let malformed = decode_malformed(&mut src);

    assert!(
        matches!(&malformed.error, MessageError::InvalidRequestLine(line) if line == "GET /health HTTP/1.1"),
        "unexpected error: {:?}",
        malformed.error
    );
    assert_eq!(malformed.cseq.as_deref(), Some("1"));
    assert!(src.is_empty());
}

#[test]
fn decode_continues_with_next_message_after_malformed_message() {
    let mut codec = MessageCodec::default();
    let mut src = buffer(b"OPTIONS\r\nCSeq: 1\r\n\r\nOPTIONS * RTSP/1.0\r\nCSeq: 2\r\n\r\n");

    let first = codec.decode(&mut src).expect("decode should succeed").expect("first message");
    let second = codec.decode(&mut src).expect("decode should succeed").expect("second message");

    assert!(first.is_err(), "unexpected message: {first:?}");
    let second = second.expect("second message should be well-formed");
    assert_eq!(second.to_string(), "OPTIONS * RTSP/1.0\nCSeq: 2");
    assert!(src.is_empty());
}

#[test]
fn decode_skips_body_of_malformed_message() {
    let mut codec = MessageCodec::default();
    let mut src = buffer(
        b"OPTIONS\r\nCSeq: 1\r\nContent-Length: 5\r\n\r\nhelloOPTIONS * RTSP/1.0\r\nCSeq: 2\r\n\r\n",
    );

    let first = codec.decode(&mut src).expect("decode should succeed").expect("first message");
    let second = codec.decode(&mut src).expect("decode should succeed").expect("second message");

    assert!(first.is_err(), "unexpected message: {first:?}");
    let second = second.expect("second message should be well-formed");
    assert_eq!(second.to_string(), "OPTIONS * RTSP/1.0\nCSeq: 2");
}

fn encode(message: Message) -> String {
    let mut dst = tokio_util::bytes::BytesMut::new();
    MessageCodec::default().encode(message, &mut dst).expect("encode should succeed");

    String::from_utf8(dst.to_vec()).expect("encoded message should be UTF-8")
}

#[test]
fn encode_writes_decoded_message() {
    let message = decode(&mut buffer(OPTIONS_REQUEST)).expect("message should be complete");

    assert_eq!(encode(message).as_bytes(), OPTIONS_REQUEST);
}

#[test]
fn decode_then_encode_returns_original_bytes() {
    let messages: [&[u8]; 8] = [
        OPTIONS_REQUEST,
        b"DESCRIBE rtsp://192.168.1.10:554/stream1 RTSP/1.0\r\nCSeq: 2\r\nAccept: application/sdp\r\nUser-Agent: MyClient/1.0\r\n\r\n",
        b"PLAY rtsp://example.com/stream?track=1 RTSP/1.0\r\nCSeq: 3\r\nSession: 12345678\r\n\r\n",
        b"ANNOUNCE rtsp://example.com/stream RTSP/1.0\r\nCSeq: 4\r\nContent-Type: application/sdp\r\nContent-Length: 13\r\n\r\nv=0\r\ns=Test\r\n",
        b"RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: 5\r\n\r\nhello",
        b"RTSP/1.0 454 Session Not Found\r\nCSeq: 4\r\n\r\n",
        b"RTSP/2.0 200 OK\r\nCSeq: 5\r\n\r\n",
        b"RTSP/1.0 200 OK\r\nContent-Length: 6\r\n\r\na\r\n\r\nb",
    ];

    for original in messages {
        let message = decode(&mut buffer(original)).expect("message should be complete");

        assert_eq!(encode(message), String::from_utf8_lossy(original));
    }
}

#[test]
fn encode_normalizes_message_so_it_decodes_to_same_fields() {
    let message =
        decode(&mut buffer(b"  options   *  rtsp/1.0\r\n  cseq :  7 \r\n\r\n")).expect("message");

    let encoded = encode(message);
    assert_eq!(encoded, "OPTIONS * RTSP/1.0\r\nCSeq: 7\r\n\r\n");

    let request = decode_request(&mut buffer(encoded.as_bytes()));
    assert_eq!(request.method(), &RequestMethod::Options);
    assert_eq!(request.version(), &Version::V1);
    assert_eq!(request.headers().get("CSeq"), Some("7"));
}

#[test]
fn encode_appends_after_existing_data_so_messages_can_be_pipelined() {
    let mut codec = MessageCodec::default();
    let mut dst = tokio_util::bytes::BytesMut::new();
    let first = decode(&mut buffer(OPTIONS_REQUEST)).expect("first message");
    let second = decode(&mut buffer(b"RTSP/1.0 200 OK\r\nCSeq: 1\r\nContent-Length: 2\r\n\r\nhi"))
        .expect("second message");

    codec.encode(first, &mut dst).expect("encode should succeed");
    codec.encode(second, &mut dst).expect("encode should succeed");

    let request = decode_request(&mut dst);
    assert_eq!(request.method(), &RequestMethod::Options);
    let response = decode_response(&mut dst);
    assert_eq!(response.body(), b"hi");
    assert!(dst.is_empty());
}
