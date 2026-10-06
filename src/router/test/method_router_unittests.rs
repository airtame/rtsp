use super::*;
use crate::message::{MessageHeaders, ParsingMode, Version};
use crate::router::Router;

fn request(start_line: &[u8]) -> Request {
    Request::parse(
        start_line,
        MessageHeaders::default(),
        tokio_util::bytes::Bytes::new(),
        ParsingMode::Lenient,
    )
    .expect("request line should be valid")
}

fn respond_with(status_code: StatusCode) -> impl RequestHandler {
    move |_: &Request| Response::new(Version::V1, status_code)
}

fn extension(method: &str) -> RequestMethod {
    RequestMethod::Extension(method.to_owned())
}

#[test]
fn handle_calls_handler_registered_for_request_method() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Setup, respond_with(StatusCode::Ok))
        .with_method(RequestMethod::Teardown, respond_with(StatusCode::ServiceUnavailable));

    let setup = router.handle(&request(b"SETUP rtsp://example.com/stream1 RTSP/1.0"));
    let teardown = router.handle(&request(b"TEARDOWN rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(setup.status_code(), 200);
    assert_eq!(teardown.status_code(), 503);
}

#[test]
fn handle_matches_extension_methods() {
    let router = MethodRouter::new()
        .with_method(extension("GET"), respond_with(StatusCode::Ok))
        .with_method(extension("POST"), respond_with(StatusCode::ServiceUnavailable));

    let get = router.handle(&request(b"GET /info RTSP/1.0"));
    let post = router.handle(&request(b"POST /info RTSP/1.0"));

    assert_eq!(get.status_code(), 200);
    assert_eq!(post.status_code(), 503);
}

#[test]
fn handle_ignores_request_path() {
    let router =
        MethodRouter::new().with_method(RequestMethod::Setup, respond_with(StatusCode::Ok));

    let response =
        router.handle(&request(b"SETUP rtsp://example.com/8279061121985366567 RTSP/1.0"));

    assert_eq!(response.status_code(), 200);
}

#[test]
fn with_method_replaces_handler_for_same_method() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Play, respond_with(StatusCode::ServiceUnavailable))
        .with_method(RequestMethod::Play, respond_with(StatusCode::Ok));

    let response = router.handle(&request(b"PLAY rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(response.status_code(), 200);
}

#[test]
fn handle_answers_unregistered_method_with_method_not_allowed() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Play, respond_with(StatusCode::Ok))
        .with_method(RequestMethod::Describe, respond_with(StatusCode::Ok));

    let response = router.handle(&request(b"RECORD rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(response.status_code(), 405);
    assert_eq!(response.reason_phrase(), "Method Not Allowed");
    assert_eq!(response.headers().get("Allow"), Some("DESCRIBE, PLAY"));
}

#[test]
fn handle_answers_method_not_allowed_with_request_version() {
    let router = MethodRouter::new();

    let rtsp_1 = router.handle(&request(b"RECORD rtsp://example.com/stream1 RTSP/1.0"));
    let rtsp_2 = router.handle(&request(b"RECORD rtsp://example.com/stream1 RTSP/2.0"));

    assert_eq!(rtsp_1.version(), &Version::V1);
    assert_eq!(rtsp_2.version(), &Version::V2);
}

#[test]
fn new_method_router_answers_with_empty_allow() {
    let response = MethodRouter::new().handle(&request(b"OPTIONS * RTSP/1.0"));

    assert_eq!(response.status_code(), 405);
    assert_eq!(response.headers().get("Allow"), Some(""));
}

#[test]
fn handle_calls_fallback_for_unregistered_method() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Play, respond_with(StatusCode::ServiceUnavailable))
        .with_fallback(respond_with(StatusCode::Ok));

    let response = router.handle(&request(b"RECORD rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(response.status_code(), 200);
}

#[test]
fn handle_prefers_registered_method_over_fallback() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Play, respond_with(StatusCode::ServiceUnavailable))
        .with_fallback(respond_with(StatusCode::Ok));

    let response = router.handle(&request(b"PLAY rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(response.status_code(), 503);
}

#[test]
fn debug_lists_registered_methods_in_order() {
    let router = MethodRouter::new()
        .with_method(RequestMethod::Teardown, respond_with(StatusCode::Ok))
        .with_method(RequestMethod::Setup, respond_with(StatusCode::Ok))
        .with_fallback(respond_with(StatusCode::Ok));

    assert_eq!(
        format!("{router:?}"),
        r#"MethodRouter { methods: ["SETUP", "TEARDOWN"], fallback: true }"#
    );
}

#[test]
fn router_fallback_dispatches_unknown_paths_by_method() {
    let router = Router::new().with_fallback(
        MethodRouter::new()
            .with_method(RequestMethod::Setup, respond_with(StatusCode::Ok))
            .with_fallback(respond_with(StatusCode::ServiceUnavailable)),
    );
    router.register("/info", respond_with(StatusCode::Created));

    let info = router.handle(&request(b"GET /info RTSP/1.0"));
    let setup = router.handle(&request(b"SETUP rtsp://example.com/8279061121985366567 RTSP/1.0"));
    let other = router.handle(&request(b"POST /unknown RTSP/1.0"));

    assert_eq!(info.status_code(), 201);
    assert_eq!(setup.status_code(), 200);
    assert_eq!(other.status_code(), 503);
}

#[test]
fn method_router_can_be_registered_for_a_path() {
    let router = Router::new();
    router.register(
        "/stream1",
        MethodRouter::new().with_method(RequestMethod::Describe, respond_with(StatusCode::Ok)),
    );

    let describe = router.handle(&request(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0"));
    let record = router.handle(&request(b"RECORD rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(describe.status_code(), 200);
    assert_eq!(record.status_code(), 405);
}
