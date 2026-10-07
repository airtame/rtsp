//! Minimal RTSP server that runs until Ctrl+C is pressed.
//!
//! ```sh
//! cargo run --example server               # listens on 127.0.0.1:8554
//! cargo run --example server 0.0.0.0:8554  # listens on a custom address
//! ```
//!
//! Send a request from another terminal with
//! `printf 'OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc 127.0.0.1 8554`. The parsed request is
//! logged, and the connection closes when `nc` exits (logged as "closed by peer"). A connection
//! that gets no complete RTSP message for 60 seconds is closed (logged as "idle timeout"), and
//! Ctrl+C closes every open connection (logged as "cancelled") before the server exits.
//! `PrintingDelegate` prints each new connection and why it closed.

const DEFAULT_ADDR: &str = "127.0.0.1:8554";
const DEFAULT_CONNECTION_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const STREAM_PATH: &str = "/stream1";
const HEALTH_PATH: &str = "/health";
const SUPPORTED_METHODS: &str = "OPTIONS, DESCRIBE";
const STREAM_SDP: &str = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=Example\r\nt=0 0\r\n";

struct StreamDescription {
    sdp: String,
}

impl rtsp::RequestHandler for StreamDescription {
    fn handle(&self, request: &rtsp::Request) -> rtsp::Response {
        rtsp::Response::new(request.version().clone(), rtsp::StatusCode::Ok)
            .with_header("Content-Type", "application/sdp")
            .with_body(self.sdp.clone())
    }
}

fn options(request: &rtsp::Request) -> rtsp::Response {
    rtsp::Response::new(request.version().clone(), rtsp::StatusCode::Ok)
        .with_header("Public", SUPPORTED_METHODS)
}

fn not_found(request: &rtsp::Request) -> rtsp::Response {
    rtsp::Response::new(request.version().clone(), rtsp::StatusCode::NotFound)
}

struct PrintingDelegate;

impl rtsp::ServerDelegate for PrintingDelegate {
    fn on_new_connection(&self, connection: rtsp::ConnectionHandle) {
        println!("New connection from {}", connection.peer_addr());
    }

    fn on_connection_closed(
        &self,
        peer_addr: std::net::SocketAddr,
        reason: rtsp::ConnectionCloseReason,
    ) {
        println!("Connection from {peer_addr} closed: {reason}");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("rtsp=debug"))
        .init();

    let addr: std::net::SocketAddr =
        std::env::args().nth(1).as_deref().unwrap_or(DEFAULT_ADDR).parse()?;

    let router = rtsp::Router::new().with_fallback(
        rtsp::MethodRouter::new()
            .with_method(rtsp::RequestMethod::Options, options)
            .with_fallback(not_found),
    );
    let stream_description = StreamDescription { sdp: STREAM_SDP.to_owned() };
    router.register(
        STREAM_PATH,
        rtsp::MethodRouter::new()
            .with_method(rtsp::RequestMethod::Options, options)
            .with_method(rtsp::RequestMethod::Describe, stream_description),
    );
    router.register(HEALTH_PATH, |request: &rtsp::Request| {
        rtsp::Response::new(request.version().clone(), rtsp::StatusCode::Ok)
    });

    let server = rtsp::Server::new()
        .with_handler(router)
        .with_delegate(PrintingDelegate)
        .with_connection_options(
            rtsp::ConnectionOptions::new().with_idle_timeout(DEFAULT_CONNECTION_IDLE_TIMEOUT),
        );

    let (handle, task) = server.bind(addr)?;
    println!("RTSP server listening on rtsp://{}, press Ctrl+C to stop", handle.local_addr());

    let task = tokio::spawn(task);

    tokio::signal::ctrl_c().await?;
    println!("Stopping RTSP server and closing open connections");
    handle.stop();
    task.await?;

    println!("RTSP server stopped");

    Ok(())
}
