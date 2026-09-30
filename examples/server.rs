//! Minimal RTSP server that listens until Ctrl+C is pressed.
//!
//! ```sh
//! cargo run --example server              # listens on 127.0.0.1:8554
//! cargo run --example server 0.0.0.0:554  # listens on a custom address
//! ```

const DEFAULT_ADDR: &str = "127.0.0.1:8554";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("rtsp=debug"))
        .init();

    let addr: std::net::SocketAddr =
        std::env::args().nth(1).as_deref().unwrap_or(DEFAULT_ADDR).parse()?;

    let server = std::sync::Arc::new(rtsp::Server::bind(addr)?);
    println!("RTSP server listening on rtsp://{addr}, press Ctrl+C to stop");

    let run = tokio::spawn({
        let server = server.clone();
        async move { server.run().await }
    });

    tokio::signal::ctrl_c().await?;
    server.stop();
    run.await?;

    println!("RTSP server stopped");

    Ok(())
}
