//! Minimal RTSP server that runs until Ctrl+C is pressed.
//!
//! ```sh
//! cargo run --example server               # listens on 127.0.0.1:8554
//! cargo run --example server 0.0.0.0:8554  # listens on a custom address
//! ```
//!
//! Open a connection from another terminal with `nc 127.0.0.1 8554`. It stays open until
//! `nc` exits (logged as "closed by peer"), nothing is typed into `nc` for 60 seconds (logged
//! as "idle timeout"), or the server is stopped with Ctrl+C, which closes every open
//! connection (logged as "cancelled") before the server exits.

const DEFAULT_ADDR: &str = "127.0.0.1:8554";
const DEFAULT_CONNECTION_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("rtsp=debug"))
        .init();

    let addr: std::net::SocketAddr =
        std::env::args().nth(1).as_deref().unwrap_or(DEFAULT_ADDR).parse()?;

    let server = std::sync::Arc::new(
        rtsp::Server::bind(addr)?.with_connection_idle_timeout(DEFAULT_CONNECTION_IDLE_TIMEOUT),
    );
    println!("RTSP server listening on rtsp://{addr}, press Ctrl+C to stop");

    let run = tokio::spawn({
        let server = server.clone();
        async move { server.run().await }
    });

    tokio::signal::ctrl_c().await?;
    println!("Stopping RTSP server and closing open connections");
    server.stop();
    run.await?;

    println!("RTSP server stopped");

    Ok(())
}
