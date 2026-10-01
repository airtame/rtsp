use super::*;

fn parse(token: &str) -> Version {
    token.parse().expect("version should be valid")
}

fn parse_err(token: &str) -> std::io::Error {
    token.parse::<Version>().expect_err("version should be invalid")
}

#[test]
fn from_str_parses_rtsp_1_0() {
    assert_eq!(parse("RTSP/1.0"), Version::V1);
}

#[test]
fn from_str_parses_rtsp_2_0() {
    assert_eq!(parse("RTSP/2.0"), Version::V2);
}

#[test]
fn from_str_returns_other_for_non_rtsp_protocol() {
    assert_eq!(parse("HTTP/1.1"), Version::Other("HTTP/1.1".to_owned()));
}

#[test]
fn from_str_accepts_multi_digit_version_numbers() {
    assert_eq!(parse("HTTP/10.20"), Version::Other("HTTP/10.20".to_owned()));
}

#[test]
fn from_str_rejects_unknown_rtsp_version() {
    let err = parse_err("RTSP/1.1");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(err.to_string(), r#"invalid version: "RTSP/1.1""#);
}

#[test]
fn from_str_rejects_empty_string() {
    assert_eq!(parse_err("").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_missing_slash() {
    assert_eq!(parse_err("RTSP").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_missing_protocol_name() {
    assert_eq!(parse_err("/1.0").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_non_alphabetic_protocol_name() {
    assert_eq!(parse_err("FOO-BAR/1.0").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_missing_minor_version() {
    assert_eq!(parse_err("HTTP/1").kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(parse_err("HTTP/1.").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_missing_major_version() {
    assert_eq!(parse_err("HTTP/.1").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_non_numeric_version() {
    assert_eq!(parse_err("HTTP/a.b").kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(parse_err("HTTP/1.2.3").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_trims_surrounding_whitespace() {
    assert_eq!(parse(" RTSP/1.0"), Version::V1);
    assert_eq!(parse("RTSP/2.0\t\r\n"), Version::V2);
}

#[test]
fn from_str_trims_surrounding_whitespace_of_other() {
    assert_eq!(parse("  HTTP/1.1 "), Version::Other("HTTP/1.1".to_owned()));
}

#[test]
fn from_str_rejects_whitespace_inside_version() {
    assert_eq!(parse_err("RTSP/ 1.0").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_rejects_whitespace_only() {
    assert_eq!(parse_err(" \t ").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_is_case_insensitive() {
    assert_eq!(parse("rtsp/1.0"), Version::V1);
    assert_eq!(parse("Rtsp/2.0"), Version::V2);
}

#[test]
fn from_str_keeps_case_of_other() {
    assert_eq!(parse("http/1.1"), Version::Other("http/1.1".to_owned()));
}

#[test]
fn from_str_rejects_unknown_rtsp_version_in_any_case() {
    assert_eq!(parse_err("rtsp/1.1").kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn from_str_error_message_shows_input_as_received() {
    assert_eq!(parse_err(" RTSP/1.1 ").to_string(), r#"invalid version: " RTSP/1.1 ""#);
}

#[test]
fn from_str_escapes_invalid_input_in_error_message() {
    assert_eq!(parse_err("RTSP/\n1.0").to_string(), r#"invalid version: "RTSP/\n1.0""#);
}

#[test]
fn display_writes_version_tokens() {
    assert_eq!(Version::V1.to_string(), "RTSP/1.0");
    assert_eq!(Version::V2.to_string(), "RTSP/2.0");
    assert_eq!(Version::Other("HTTP/1.1".to_owned()).to_string(), "HTTP/1.1");
}

#[test]
fn display_and_from_str_round_trip() {
    for version in [Version::V1, Version::V2, Version::Other("HTTP/1.1".to_owned())] {
        assert_eq!(parse(&version.to_string()), version);
    }
}
