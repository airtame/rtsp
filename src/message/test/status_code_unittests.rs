use super::*;

const RFC_2326_STATUS_CODES: [(StatusCode, u16, &str); 44] = [
    (StatusCode::Continue, 100, "Continue"),
    (StatusCode::Ok, 200, "OK"),
    (StatusCode::Created, 201, "Created"),
    (StatusCode::LowOnStorageSpace, 250, "Low on Storage Space"),
    (StatusCode::MultipleChoices, 300, "Multiple Choices"),
    (StatusCode::MovedPermanently, 301, "Moved Permanently"),
    (StatusCode::MovedTemporarily, 302, "Moved Temporarily"),
    (StatusCode::SeeOther, 303, "See Other"),
    (StatusCode::NotModified, 304, "Not Modified"),
    (StatusCode::UseProxy, 305, "Use Proxy"),
    (StatusCode::BadRequest, 400, "Bad Request"),
    (StatusCode::Unauthorized, 401, "Unauthorized"),
    (StatusCode::PaymentRequired, 402, "Payment Required"),
    (StatusCode::Forbidden, 403, "Forbidden"),
    (StatusCode::NotFound, 404, "Not Found"),
    (StatusCode::MethodNotAllowed, 405, "Method Not Allowed"),
    (StatusCode::NotAcceptable, 406, "Not Acceptable"),
    (StatusCode::ProxyAuthenticationRequired, 407, "Proxy Authentication Required"),
    (StatusCode::RequestTimeout, 408, "Request Time-out"),
    (StatusCode::Gone, 410, "Gone"),
    (StatusCode::LengthRequired, 411, "Length Required"),
    (StatusCode::PreconditionFailed, 412, "Precondition Failed"),
    (StatusCode::RequestEntityTooLarge, 413, "Request Entity Too Large"),
    (StatusCode::RequestUriTooLarge, 414, "Request-URI Too Large"),
    (StatusCode::UnsupportedMediaType, 415, "Unsupported Media Type"),
    (StatusCode::ParameterNotUnderstood, 451, "Parameter Not Understood"),
    (StatusCode::ConferenceNotFound, 452, "Conference Not Found"),
    (StatusCode::NotEnoughBandwidth, 453, "Not Enough Bandwidth"),
    (StatusCode::SessionNotFound, 454, "Session Not Found"),
    (StatusCode::MethodNotValidInThisState, 455, "Method Not Valid in This State"),
    (StatusCode::HeaderFieldNotValidForResource, 456, "Header Field Not Valid for Resource"),
    (StatusCode::InvalidRange, 457, "Invalid Range"),
    (StatusCode::ParameterIsReadOnly, 458, "Parameter Is Read-Only"),
    (StatusCode::AggregateOperationNotAllowed, 459, "Aggregate operation not allowed"),
    (StatusCode::OnlyAggregateOperationAllowed, 460, "Only aggregate operation allowed"),
    (StatusCode::UnsupportedTransport, 461, "Unsupported transport"),
    (StatusCode::DestinationUnreachable, 462, "Destination unreachable"),
    (StatusCode::InternalServerError, 500, "Internal Server Error"),
    (StatusCode::NotImplemented, 501, "Not Implemented"),
    (StatusCode::BadGateway, 502, "Bad Gateway"),
    (StatusCode::ServiceUnavailable, 503, "Service Unavailable"),
    (StatusCode::GatewayTimeout, 504, "Gateway Time-out"),
    (StatusCode::RtspVersionNotSupported, 505, "RTSP Version not supported"),
    (StatusCode::OptionNotSupported, 551, "Option not supported"),
];

#[test]
fn code_matches_rfc_2326() {
    for (status_code, code, _) in RFC_2326_STATUS_CODES {
        assert_eq!(status_code.code(), code, "{status_code:?}");
    }
}

#[test]
fn reason_phrase_matches_rfc_2326() {
    for (status_code, _, reason_phrase) in RFC_2326_STATUS_CODES {
        assert_eq!(status_code.reason_phrase(), reason_phrase, "{status_code:?}");
    }
}

#[test]
fn try_from_returns_status_code_for_each_standard_code() {
    for (status_code, code, _) in RFC_2326_STATUS_CODES {
        assert_eq!(StatusCode::try_from(code), Ok(status_code));
    }
}

#[test]
fn try_from_returns_code_for_unknown_status_code() {
    for code in [0, 99, 202, 299, 409, 999, 1000] {
        assert_eq!(StatusCode::try_from(code), Err(code));
    }
}

#[test]
fn converts_into_its_code() {
    let code: u16 = StatusCode::SessionNotFound.into();

    assert_eq!(code, 454);
}
