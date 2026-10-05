use super::*;
use crate::message::{
    MessageHeaders, ParsingMode, Request, RequestMethod, Response, StatusCode, Version,
};

fn request(start_line: &[u8]) -> Request {
    request_with_headers(start_line, "")
}

fn respond_with(status_code: StatusCode) -> impl RequestHandler {
    move |_: &Request| Response::new(Version::V1, status_code)
}

fn status_code(router: &Router, path: &str) -> Option<u16> {
    let handler = router.get(path)?;

    Some(handler.handle(&request(b"OPTIONS * RTSP/1.0")).status_code())
}

#[test]
fn new_router_has_no_handlers() {
    assert!(Router::new().get("/stream1").is_none());
}

#[test]
fn register_adds_handler() {
    let router = Router::new();

    assert!(router.register("/stream1", respond_with(StatusCode::Ok)).is_none());

    assert_eq!(status_code(&router, "/stream1"), Some(200));
}

#[test]
fn register_keeps_handlers_for_different_paths() {
    let router = Router::new();

    router.register("/stream1", respond_with(StatusCode::Ok));
    router.register("/stream2", respond_with(StatusCode::NotFound));

    assert_eq!(status_code(&router, "/stream1"), Some(200));
    assert_eq!(status_code(&router, "/stream2"), Some(404));
}

#[test]
fn register_replaces_and_returns_previous_handler() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    let previous = router
        .register("/stream1", respond_with(StatusCode::ServiceUnavailable))
        .expect("a handler was already registered");

    assert_eq!(previous.handle(&request(b"OPTIONS * RTSP/1.0")).status_code(), 200);
    assert_eq!(status_code(&router, "/stream1"), Some(503));
}

#[test]
fn unregister_removes_and_returns_handler() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));
    router.register("/stream2", respond_with(StatusCode::Ok));

    let removed = router.unregister("/stream1").expect("a handler was registered");

    assert_eq!(removed.handle(&request(b"OPTIONS * RTSP/1.0")).status_code(), 200);
    assert_eq!(status_code(&router, "/stream1"), None);
    assert_eq!(status_code(&router, "/stream2"), Some(200));
}

#[test]
fn unregister_returns_none_for_unknown_path() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    assert!(router.unregister("/stream2").is_none());
    assert_eq!(status_code(&router, "/stream1"), Some(200));
}

#[test]
fn get_matches_paths_exactly() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    assert!(router.get("/stream1/").is_none());
    assert!(router.get("/Stream1").is_none());
    assert!(router.get("/stream").is_none());
}

#[test]
fn closure_handler_receives_request() {
    let router = Router::new();
    router.register("/stream1", |request: &Request| match request.method() {
        RequestMethod::Describe => Response::new(Version::V1, StatusCode::Ok),
        _ => Response::new(Version::V1, StatusCode::MethodNotAllowed),
    });
    let handler = router.get("/stream1").expect("a handler was registered");

    let describe = request(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0");
    let record = request(b"RECORD rtsp://example.com/stream1 RTSP/1.0");

    assert_eq!(handler.handle(&describe).status_code(), 200);
    assert_eq!(handler.handle(&record).status_code(), 405);
}

#[test]
fn struct_handler_keeps_its_own_state() {
    struct CountingHandler {
        handled: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl RequestHandler for CountingHandler {
        fn handle(&self, _request: &Request) -> Response {
            self.handled.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Response::new(Version::V1, StatusCode::Ok)
        }
    }

    let handled = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let router = Router::new();
    router.register("/stream1", CountingHandler { handled: handled.clone() });
    let handler = router.get("/stream1").expect("a handler was registered");

    handler.handle(&request(b"OPTIONS * RTSP/1.0"));
    handler.handle(&request(b"OPTIONS * RTSP/1.0"));

    assert_eq!(handled.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn clones_share_handlers() {
    let router = Router::new();
    let clone = router.clone();

    clone.register("/stream1", respond_with(StatusCode::Ok));
    assert_eq!(status_code(&router, "/stream1"), Some(200));

    router.unregister("/stream1");
    assert!(clone.get("/stream1").is_none());
}

#[test]
fn handlers_can_be_registered_from_another_thread() {
    let router = Router::new();

    std::thread::spawn({
        let router = router.clone();
        move || router.register("/stream1", respond_with(StatusCode::Ok))
    })
    .join()
    .expect("registering thread panicked");

    assert_eq!(status_code(&router, "/stream1"), Some(200));
}

#[test]
fn debug_lists_registered_paths_in_order() {
    let router = Router::new();
    router.register("/stream2", respond_with(StatusCode::Ok));
    router.register("/stream1", respond_with(StatusCode::Ok));

    assert_eq!(format!("{router:?}"), r#"Router { paths: ["/stream1", "/stream2"] }"#);
}

fn request_with_headers(start_line: &[u8], header_lines: &str) -> Request {
    let headers =
        MessageHeaders::try_from(header_lines.as_bytes()).expect("headers should be valid");

    Request::parse(start_line, headers, tokio_util::bytes::Bytes::new(), ParsingMode::Lenient)
        .expect("request line should be valid")
}

#[test]
fn route_calls_handler_registered_for_request_path() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));
    router.register("/stream2", respond_with(StatusCode::ServiceUnavailable));

    let response = router.route(&request(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(response.status_code(), 200);
}

#[test]
fn route_passes_request_to_handler() {
    let router = Router::new();
    router.register("/stream1", |request: &Request| match request.method() {
        RequestMethod::Play => Response::new(Version::V1, StatusCode::Ok),
        _ => Response::new(Version::V1, StatusCode::MethodNotAllowed),
    });

    let play = router.route(&request(b"PLAY rtsp://example.com/stream1 RTSP/1.0"));
    let record = router.route(&request(b"RECORD rtsp://example.com/stream1 RTSP/1.0"));

    assert_eq!(play.status_code(), 200);
    assert_eq!(record.status_code(), 405);
}

#[test]
fn route_ignores_query_when_matching_path() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    let response = router.route(&request(b"PLAY rtsp://example.com/stream1?track=1 RTSP/1.0"));

    assert_eq!(response.status_code(), 200);
}

#[test]
fn route_returns_not_found_for_unknown_path() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    let response = router.route(&request(b"DESCRIBE rtsp://example.com/stream2 RTSP/1.0"));

    assert_eq!(response.status_code(), 404);
    assert_eq!(response.reason_phrase(), "Not Found");
}

#[test]
fn route_returns_not_found_without_handlers() {
    let response = Router::new().route(&request(b"OPTIONS * RTSP/1.0"));

    assert_eq!(response.status_code(), 404);
}

#[test]
fn route_answers_not_found_with_request_version() {
    let router = Router::new();

    let rtsp_1 = router.route(&request(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0"));
    let rtsp_2 = router.route(&request(b"DESCRIBE rtsp://example.com/stream1 RTSP/2.0"));

    assert_eq!(rtsp_1.version(), &Version::V1);
    assert_eq!(rtsp_2.version(), &Version::V2);
}

#[test]
fn route_leaves_cseq_to_the_connection() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    let response = router
        .route(&request_with_headers(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0", "CSeq: 7"));

    assert_eq!(response.headers().get("CSeq"), None);
}

#[test]
fn handle_routes_request_like_route() {
    let router = Router::new();
    router.register("/stream1", respond_with(StatusCode::Ok));

    let found = router.handle(&request(b"DESCRIBE rtsp://example.com/stream1 RTSP/1.0"));
    let not_found = router.handle(&request(b"DESCRIBE rtsp://example.com/stream2 RTSP/1.0"));

    assert_eq!(found.status_code(), 200);
    assert_eq!(not_found.status_code(), 404);
}
