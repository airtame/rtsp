use super::*;

fn decode(value: &str) -> SessionHeader {
    SessionHeader::decode(value).expect("Session should be valid")
}

fn assert_invalid(value: &str) {
    let err = SessionHeader::decode(value).expect_err("Session should be invalid");

    assert!(
        matches!(&err, MessageError::InvalidHeaderValue(MessageHeaderName::Session, rejected) if rejected == value),
        "unexpected error for {value:?}: {err:?}"
    );
}

fn session(id: &str, timeout: Option<u64>) -> SessionHeader {
    SessionHeader { id: id.to_owned(), timeout: timeout.map(std::time::Duration::from_secs) }
}

#[test]
fn name_is_session() {
    assert!(matches!(SessionHeader::NAME, MessageHeaderName::Session));
}

#[test]
fn decode_reads_id_without_timeout() {
    assert_eq!(decode("12345678"), session("12345678", None));
}

#[test]
fn decode_reads_id_and_timeout() {
    assert_eq!(decode("12345678;timeout=60"), session("12345678", Some(60)));
}

#[test]
fn decode_reads_timeout_in_any_case_and_with_spaces() {
    assert_eq!(decode("12345678 ; Timeout = 30"), session("12345678", Some(30)));
}

#[test]
fn decode_ignores_unknown_parameters() {
    assert_eq!(decode("12345678;x-custom=1;timeout=45"), session("12345678", Some(45)));
}

#[test]
fn decode_rejects_empty_id() {
    for value in ["", " ", ";timeout=60"] {
        assert_invalid(value);
    }
}

#[test]
fn decode_rejects_timeout_that_is_not_a_number() {
    for value in ["12345678;timeout=", "12345678;timeout=abc", "12345678;timeout=-5"] {
        assert_invalid(value);
    }
}

#[test]
fn decode_rejects_parameter_without_value() {
    assert_invalid("12345678;timeout");
}

#[test]
fn encode_writes_id_without_timeout() {
    assert_eq!(session("12345678", None).encode(), "12345678");
}

#[test]
fn encode_writes_id_and_timeout_in_seconds() {
    assert_eq!(session("12345678", Some(60)).encode(), "12345678;timeout=60");
}

#[test]
fn encode_and_decode_round_trip() {
    for header in [session("12345678", None), session("abc-DEF_9", Some(120))] {
        assert_eq!(decode(&header.encode()), header);
    }
}
