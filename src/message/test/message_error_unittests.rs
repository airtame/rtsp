use super::*;

#[test]
fn display_io_includes_underlying_error() {
    let err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset by test");

    assert_eq!(MessageError::Io(err).to_string(), "I/O error: reset by test");
}

#[test]
fn display_invalid_content_length_quotes_value() {
    let err = MessageError::InvalidContentLength("abc".to_owned());

    assert_eq!(err.to_string(), r#"invalid Content-Length: "abc""#);
}

#[test]
fn display_invalid_encoding_quotes_text() {
    let err = MessageError::InvalidEncoding("CSeq: \u{fffd}".to_owned());

    assert_eq!(err.to_string(), "invalid encoding, expected UTF-8: \"CSeq: \u{fffd}\"");
}

#[test]
fn display_invalid_header_quotes_line() {
    let err = MessageError::InvalidHeader("CSeq 1".to_owned());

    assert_eq!(err.to_string(), r#"invalid header: "CSeq 1""#);
}

#[test]
fn display_invalid_request_line_quotes_line() {
    let err = MessageError::InvalidRequestLine("OPTIONS *".to_owned());

    assert_eq!(err.to_string(), r#"invalid request line: "OPTIONS *""#);
}

#[test]
fn display_invalid_status_line_quotes_line() {
    let err = MessageError::InvalidStatusLine("RTSP/1.0".to_owned());

    assert_eq!(err.to_string(), r#"invalid status line: "RTSP/1.0""#);
}

#[test]
fn display_missing_header_quotes_name() {
    let err = MessageError::MissingHeader("CSeq".to_owned());

    assert_eq!(err.to_string(), r#"missing header: "CSeq""#);
}

#[test]
fn display_escapes_control_characters() {
    let err = MessageError::InvalidHeader("a\nb\x1b[31m".to_owned());

    assert_eq!(err.to_string(), r#"invalid header: "a\nb\u{1b}[31m""#);
}

#[test]
fn converts_into_boxed_std_error() {
    let err: Box<dyn std::error::Error> = MessageError::MissingHeader("CSeq".to_owned()).into();

    assert_eq!(err.to_string(), r#"missing header: "CSeq""#);
}

#[test]
fn from_io_error_wraps_it_in_io_variant() {
    let err = MessageError::from(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));

    assert!(
        matches!(&err, MessageError::Io(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof),
        "unexpected error: {err:?}"
    );
}
