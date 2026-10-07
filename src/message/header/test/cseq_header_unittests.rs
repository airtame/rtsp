use super::*;

#[test]
fn name_is_cseq() {
    assert!(matches!(CSeqHeader::NAME, MessageHeaderName::CSeq));
}

#[test]
fn decode_reads_number() {
    assert_eq!(CSeqHeader::decode("42").expect("CSeq should be valid"), CSeqHeader(42));
}

#[test]
fn decode_reads_largest_nine_digit_number() {
    let cseq = CSeqHeader::decode("999999999").expect("CSeq should be valid");

    assert_eq!(cseq, CSeqHeader(999_999_999));
}

#[test]
fn decode_rejects_value_that_is_not_a_number() {
    for value in ["", "abc", "1 2", "-1", "1\nX-Injected: yes", "99999999999"] {
        let err = CSeqHeader::decode(value).expect_err("CSeq should be invalid");

        assert!(
            matches!(&err, MessageError::InvalidHeaderValue(MessageHeaderName::CSeq, rejected) if rejected == value),
            "unexpected error for {value:?}: {err:?}"
        );
    }
}

#[test]
fn encode_writes_number() {
    assert_eq!(CSeqHeader(7).encode(), "7");
}
