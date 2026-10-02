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
const STREAM_METHODS: &str = "OPTIONS, DESCRIBE";
const STREAM_SDP: &str = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=Example\r\nt=0 0\r\n";

struct ExampleStream {
    sdp: String,
}

impl rtsp::RequestHandler for ExampleStream {
    fn handle(&self, request: &rtsp::Request) -> rtsp::Response {
        let version = request.version().clone();

        match request.method() {
            rtsp::RequestMethod::Options => rtsp::Response::new(version, rtsp::StatusCode::Ok)
                .with_header("Public", STREAM_METHODS),
            rtsp::RequestMethod::Describe => rtsp::Response::new(version, rtsp::StatusCode::Ok)
                .with_header("Content-Type", "application/sdp")
                .with_body(self.sdp.clone()),
            _ => rtsp::Response::new(version, rtsp::StatusCode::MethodNotAllowed)
                .with_header("Allow", STREAM_METHODS),
        }
    }
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

    let router = rtsp::Router::new();
    router.register(STREAM_PATH, ExampleStream { sdp: STREAM_SDP.to_owned() });
    router.register(HEALTH_PATH, |request: &rtsp::Request| {
        rtsp::Response::new(request.version().clone(), rtsp::StatusCode::Ok)
    });

    let server = std::sync::Arc::new(
        rtsp::Server::bind(addr)?
            .with_connection_idle_timeout(DEFAULT_CONNECTION_IDLE_TIMEOUT)
            .with_router(router),
    );
    println!("RTSP server listening on rtsp://{addr}, press Ctrl+C to stop");

    let run = tokio::spawn({
        let server = server.clone();
        async move { server.run(&PrintingDelegate).await }
    });

    tokio::signal::ctrl_c().await?;
    println!("Stopping RTSP server and closing open connections");
    server.stop();
    run.await?;

    println!("RTSP server stopped");

    Ok(())
}
