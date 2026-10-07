use super::*;

const STANDARD_NAMES: [(&str, MessageHeaderName); 62] = [
    ("Accept", MessageHeaderName::Accept),
    ("Accept-Credentials", MessageHeaderName::AcceptCredentials),
    ("Accept-Encoding", MessageHeaderName::AcceptEncoding),
    ("Accept-Language", MessageHeaderName::AcceptLanguage),
    ("Accept-Ranges", MessageHeaderName::AcceptRanges),
    ("Allow", MessageHeaderName::Allow),
    ("Authentication-Info", MessageHeaderName::AuthenticationInfo),
    ("Authorization", MessageHeaderName::Authorization),
    ("Bandwidth", MessageHeaderName::Bandwidth),
    ("Blocksize", MessageHeaderName::Blocksize),
    ("Cache-Control", MessageHeaderName::CacheControl),
    ("Conference", MessageHeaderName::Conference),
    ("Connection", MessageHeaderName::Connection),
    ("Connection-Credentials", MessageHeaderName::ConnectionCredentials),
    ("Content-Base", MessageHeaderName::ContentBase),
    ("Content-Encoding", MessageHeaderName::ContentEncoding),
    ("Content-Language", MessageHeaderName::ContentLanguage),
    ("Content-Length", MessageHeaderName::ContentLength),
    ("Content-Location", MessageHeaderName::ContentLocation),
    ("Content-Type", MessageHeaderName::ContentType),
    ("CSeq", MessageHeaderName::CSeq),
    ("Date", MessageHeaderName::Date),
    ("Expires", MessageHeaderName::Expires),
    ("From", MessageHeaderName::From),
    ("Host", MessageHeaderName::Host),
    ("If-Match", MessageHeaderName::IfMatch),
    ("If-Modified-Since", MessageHeaderName::IfModifiedSince),
    ("If-None-Match", MessageHeaderName::IfNoneMatch),
    ("Last-Modified", MessageHeaderName::LastModified),
    ("Location", MessageHeaderName::Location),
    ("Media-Properties", MessageHeaderName::MediaProperties),
    ("Media-Range", MessageHeaderName::MediaRange),
    ("MTag", MessageHeaderName::MTag),
    ("Notify-Reason", MessageHeaderName::NotifyReason),
    ("Pipelined-Requests", MessageHeaderName::PipelinedRequests),
    ("Proxy-Authenticate", MessageHeaderName::ProxyAuthenticate),
    ("Proxy-Authentication-Info", MessageHeaderName::ProxyAuthenticationInfo),
    ("Proxy-Authorization", MessageHeaderName::ProxyAuthorization),
    ("Proxy-Require", MessageHeaderName::ProxyRequire),
    ("Proxy-Supported", MessageHeaderName::ProxySupported),
    ("Public", MessageHeaderName::Public),
    ("Range", MessageHeaderName::Range),
    ("Referer", MessageHeaderName::Referer),
    ("Referrer", MessageHeaderName::Referrer),
    ("Request-Status", MessageHeaderName::RequestStatus),
    ("Require", MessageHeaderName::Require),
    ("Retry-After", MessageHeaderName::RetryAfter),
    ("RTP-Info", MessageHeaderName::RtpInfo),
    ("Scale", MessageHeaderName::Scale),
    ("Seek-Style", MessageHeaderName::SeekStyle),
    ("Server", MessageHeaderName::Server),
    ("Session", MessageHeaderName::Session),
    ("Speed", MessageHeaderName::Speed),
    ("Supported", MessageHeaderName::Supported),
    ("Terminate-Reason", MessageHeaderName::TerminateReason),
    ("Timestamp", MessageHeaderName::Timestamp),
    ("Transport", MessageHeaderName::Transport),
    ("Unsupported", MessageHeaderName::Unsupported),
    ("User-Agent", MessageHeaderName::UserAgent),
    ("Vary", MessageHeaderName::Vary),
    ("Via", MessageHeaderName::Via),
    ("WWW-Authenticate", MessageHeaderName::WwwAuthenticate),
];

fn parse(name: &str) -> MessageHeaderName {
    name.parse().expect("parsing a header name never fails")
}

fn extension(name: &str) -> MessageHeaderName {
    MessageHeaderName::Extension(name.to_owned())
}

#[test]
fn from_str_parses_standard_names() {
    for (name, header_name) in STANDARD_NAMES {
        assert!(!matches!(parse(name), MessageHeaderName::Extension(_)), "name: {name}");
        assert_eq!(parse(name), header_name, "name: {name}");
    }
}

#[test]
fn as_str_returns_standard_spelling() {
    for (name, header_name) in STANDARD_NAMES {
        assert_eq!(header_name.as_str(), name);
    }
}

#[test]
fn display_writes_standard_spelling() {
    for (name, header_name) in STANDARD_NAMES {
        assert_eq!(header_name.to_string(), name);
    }
}

#[test]
fn display_and_from_str_round_trip() {
    for header_name in STANDARD_NAMES.map(|(_, header_name)| header_name) {
        assert_eq!(parse(&header_name.to_string()).as_str(), header_name.as_str());
    }
}

#[test]
fn from_str_is_case_insensitive() {
    assert!(matches!(parse("cseq"), MessageHeaderName::CSeq));
    assert!(matches!(parse("CSEQ"), MessageHeaderName::CSeq));
    assert!(matches!(parse("www-authenticate"), MessageHeaderName::WwwAuthenticate));
    assert!(matches!(parse("Rtp-Info"), MessageHeaderName::RtpInfo));
}

#[test]
fn display_writes_standard_spelling_whatever_the_parsed_case() {
    assert_eq!(parse("cseq").to_string(), "CSeq");
    assert_eq!(parse("WWW-AUTHENTICATE").to_string(), "WWW-Authenticate");
}

#[test]
fn from_str_returns_extension_for_unknown_name() {
    assert!(matches!(parse("X-Custom"), MessageHeaderName::Extension(name) if name == "X-Custom"));
}

#[test]
fn display_writes_extension_name_unchanged() {
    assert_eq!(extension("x-Custom").to_string(), "x-Custom");
}

#[test]
fn from_str_keeps_case_of_extension() {
    let name = parse("x-cUSTOM");

    assert!(matches!(&name, MessageHeaderName::Extension(name) if name == "x-cUSTOM"));
}

#[test]
fn from_str_trims_surrounding_whitespace() {
    let name = parse("  X-Custom  ");

    assert!(matches!(parse(" Session\t"), MessageHeaderName::Session));
    assert!(matches!(&name, MessageHeaderName::Extension(name) if name == "X-Custom"));
}

#[test]
fn from_str_returns_extension_for_empty_string() {
    assert!(matches!(parse(""), MessageHeaderName::Extension(name) if name.is_empty()));
}

#[test]
fn from_string_matches_from_str() {
    let name = MessageHeaderName::from("Transport".to_owned());

    assert!(matches!(name, MessageHeaderName::Transport));
    assert_eq!(MessageHeaderName::from("X-Custom".to_owned()), extension("X-Custom"));
}

#[test]
fn eq_ignores_case_of_extension() {
    assert_eq!(extension("X-Custom"), extension("x-custom"));
    assert_ne!(extension("X-Custom"), extension("X-Other"));
}

#[test]
fn eq_treats_extension_spelled_like_standard_name_as_that_name() {
    assert_eq!(extension("cseq"), MessageHeaderName::CSeq);
}

#[test]
fn hash_ignores_case() {
    let names =
        std::collections::HashSet::from([extension("X-Custom"), MessageHeaderName::Session]);

    assert!(names.contains(&extension("x-CUSTOM")));
    assert!(names.contains(&extension("SESSION")));
}
