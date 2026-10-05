use super::*;

const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[test]
fn new_equals_default() {
    assert_eq!(ConnectionOptions::new(), ConnectionOptions::default());
}

#[test]
fn default_has_no_idle_timeout() {
    assert_eq!(ConnectionOptions::default().idle_timeout(), None);
}

#[test]
fn default_uses_strict_parsing_mode() {
    assert_eq!(ConnectionOptions::default().parsing_mode(), ParsingMode::Strict);
}

#[test]
fn with_idle_timeout_sets_idle_timeout() {
    let options = ConnectionOptions::new().with_idle_timeout(IDLE_TIMEOUT);

    assert_eq!(options.idle_timeout(), Some(IDLE_TIMEOUT));
}

#[test]
fn with_idle_timeout_replaces_previous_idle_timeout() {
    let options = ConnectionOptions::new()
        .with_idle_timeout(std::time::Duration::from_secs(10))
        .with_idle_timeout(IDLE_TIMEOUT);

    assert_eq!(options.idle_timeout(), Some(IDLE_TIMEOUT));
}

#[test]
fn with_idle_timeout_keeps_parsing_mode() {
    let options = ConnectionOptions::new()
        .with_parsing_mode(ParsingMode::Lenient)
        .with_idle_timeout(IDLE_TIMEOUT);

    assert_eq!(options.parsing_mode(), ParsingMode::Lenient);
}

#[test]
fn with_parsing_mode_sets_parsing_mode() {
    let options = ConnectionOptions::new().with_parsing_mode(ParsingMode::Lenient);

    assert_eq!(options.parsing_mode(), ParsingMode::Lenient);
}

#[test]
fn with_parsing_mode_replaces_previous_parsing_mode() {
    let options = ConnectionOptions::new()
        .with_parsing_mode(ParsingMode::Lenient)
        .with_parsing_mode(ParsingMode::Strict);

    assert_eq!(options.parsing_mode(), ParsingMode::Strict);
}

#[test]
fn with_parsing_mode_keeps_idle_timeout() {
    let options = ConnectionOptions::new()
        .with_idle_timeout(IDLE_TIMEOUT)
        .with_parsing_mode(ParsingMode::Lenient);

    assert_eq!(options.idle_timeout(), Some(IDLE_TIMEOUT));
}

#[test]
fn debug_shows_default_options() {
    assert_eq!(
        format!("{:?}", ConnectionOptions::default()),
        "ConnectionOptions { idle_timeout: None, parsing_mode: Strict }"
    );
}

#[test]
fn debug_shows_configured_options() {
    let options = ConnectionOptions::new()
        .with_idle_timeout(IDLE_TIMEOUT)
        .with_parsing_mode(ParsingMode::Lenient);

    assert_eq!(
        format!("{options:?}"),
        "ConnectionOptions { idle_timeout: Some(30s), parsing_mode: Lenient }"
    );
}
