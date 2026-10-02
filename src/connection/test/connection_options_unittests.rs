use super::*;

const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const PEER_ADDR: std::net::SocketAddr =
    std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8554);

type Calls = std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>;

fn recording_hook(calls: &Calls) -> impl Fn(std::net::SocketAddr) + Send + Sync + 'static {
    let calls = calls.clone();
    move |peer_addr| calls.lock().unwrap().push(peer_addr)
}

#[test]
fn new_equals_default() {
    assert_eq!(
        format!("{:?}", ConnectionOptions::new()),
        format!("{:?}", ConnectionOptions::default())
    );
}

#[test]
fn default_has_no_activity_hook() {
    assert!(ConnectionOptions::default().activity_hook().is_none());
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
fn with_activity_hook_sets_activity_hook() {
    let calls = Calls::default();
    let options = ConnectionOptions::new().with_activity_hook(recording_hook(&calls));

    let activity_hook = options.activity_hook().expect("activity hook should be set");
    activity_hook(PEER_ADDR);

    assert_eq!(*calls.lock().unwrap(), vec![PEER_ADDR]);
}

#[test]
fn with_activity_hook_replaces_previous_activity_hook() {
    let first = Calls::default();
    let second = Calls::default();
    let options = ConnectionOptions::new()
        .with_activity_hook(recording_hook(&first))
        .with_activity_hook(recording_hook(&second));

    let activity_hook = options.activity_hook().expect("activity hook should be set");
    activity_hook(PEER_ADDR);

    assert!(first.lock().unwrap().is_empty());
    assert_eq!(*second.lock().unwrap(), vec![PEER_ADDR]);
}

#[test]
fn with_activity_hook_keeps_idle_timeout_and_parsing_mode() {
    let options = ConnectionOptions::new()
        .with_idle_timeout(IDLE_TIMEOUT)
        .with_parsing_mode(ParsingMode::Lenient)
        .with_activity_hook(|_| {});

    assert_eq!(options.idle_timeout(), Some(IDLE_TIMEOUT));
    assert_eq!(options.parsing_mode(), ParsingMode::Lenient);
}

#[test]
fn clone_shares_activity_hook() {
    let calls = Calls::default();
    let options = ConnectionOptions::new().with_activity_hook(recording_hook(&calls));
    let cloned = options.clone();

    let activity_hook = cloned.activity_hook().expect("activity hook should be set");
    activity_hook(PEER_ADDR);

    assert_eq!(*calls.lock().unwrap(), vec![PEER_ADDR]);
}

#[test]
fn debug_shows_default_options() {
    assert_eq!(
        format!("{:?}", ConnectionOptions::default()),
        "ConnectionOptions { idle_timeout: None, parsing_mode: Strict, activity_hook: false }"
    );
}

#[test]
fn debug_shows_configured_options() {
    let options = ConnectionOptions::new()
        .with_idle_timeout(IDLE_TIMEOUT)
        .with_parsing_mode(ParsingMode::Lenient)
        .with_activity_hook(|_| {});

    assert_eq!(
        format!("{options:?}"),
        "ConnectionOptions { idle_timeout: Some(30s), parsing_mode: Lenient, activity_hook: true }"
    );
}
