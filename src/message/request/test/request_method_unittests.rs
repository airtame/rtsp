use super::*;

const STANDARD_METHODS: [(&str, RequestMethod); 11] = [
    ("DESCRIBE", RequestMethod::Describe),
    ("ANNOUNCE", RequestMethod::Announce),
    ("GET_PARAMETER", RequestMethod::GetParameter),
    ("OPTIONS", RequestMethod::Options),
    ("PAUSE", RequestMethod::Pause),
    ("PLAY", RequestMethod::Play),
    ("RECORD", RequestMethod::Record),
    ("REDIRECT", RequestMethod::Redirect),
    ("SETUP", RequestMethod::Setup),
    ("SET_PARAMETER", RequestMethod::SetParameter),
    ("TEARDOWN", RequestMethod::Teardown),
];

fn parse(token: &str) -> RequestMethod {
    token.parse().expect("parsing a request method never fails")
}

#[test]
fn from_str_parses_standard_methods() {
    for (token, method) in STANDARD_METHODS {
        assert_eq!(parse(token), method, "token: {token}");
    }
}

#[test]
fn display_writes_standard_method_tokens() {
    for (token, method) in STANDARD_METHODS {
        assert_eq!(method.to_string(), token);
    }
}

#[test]
fn from_str_returns_extension_for_unknown_method() {
    assert_eq!(parse("PLAY_NOTIFY"), RequestMethod::Extension("PLAY_NOTIFY".to_owned()));
}

#[test]
fn display_writes_extension_token_unchanged() {
    assert_eq!(RequestMethod::Extension("X_CUSTOM".to_owned()).to_string(), "X_CUSTOM");
}

#[test]
fn display_and_from_str_round_trip() {
    let extension = RequestMethod::Extension("X_CUSTOM".to_owned());

    for method in STANDARD_METHODS.map(|(_, method)| method).into_iter().chain([extension]) {
        assert_eq!(parse(&method.to_string()), method);
    }
}

#[test]
fn from_str_is_case_insensitive() {
    assert_eq!(parse("options"), RequestMethod::Options);
    assert_eq!(parse("Options"), RequestMethod::Options);
    assert_eq!(parse("set_Parameter"), RequestMethod::SetParameter);
}

#[test]
fn from_str_keeps_case_of_extension() {
    assert_eq!(parse("x_Custom"), RequestMethod::Extension("x_Custom".to_owned()));
}

#[test]
fn from_str_trims_surrounding_whitespace() {
    assert_eq!(parse(" OPTIONS\t"), RequestMethod::Options);
    assert_eq!(parse("\r\n teardown "), RequestMethod::Teardown);
}

#[test]
fn from_str_trims_surrounding_whitespace_of_extension() {
    assert_eq!(parse("  X_CUSTOM  "), RequestMethod::Extension("X_CUSTOM".to_owned()));
}

#[test]
fn from_str_returns_extension_for_empty_string() {
    assert_eq!(parse(""), RequestMethod::Extension(String::new()));
}

#[test]
fn from_str_returns_empty_extension_for_whitespace_only() {
    assert_eq!(parse(" \t "), RequestMethod::Extension(String::new()));
}
