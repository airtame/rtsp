const DEFAULT_ADDR: &str = "127.0.0.1:8554";
const STREAM_PATH: &str = "/stream1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("rtsp=debug"))
        .init();

    let addr: std::net::SocketAddr =
        std::env::args().nth(1).as_deref().unwrap_or(DEFAULT_ADDR).parse()?;

    let client = rtsp::Client::new()
        .with_handler(|request: &rtsp::Request| {
            rtsp::Response::new(request.version().clone(), rtsp::StatusCode::Ok)
        })
        .with_connection_options(
            rtsp::ConnectionOptions::new()
                .with_activity_hook(|peer_addr| println!("Message received from {peer_addr}")),
        );

    let (handle, task) = client.connect(addr).await?;
    println!("Connected to rtsp://{}, press Ctrl+C to disconnect", handle.peer_addr());

    let mut task = tokio::spawn(task);

    let stream_uri = format!("rtsp://{addr}{STREAM_PATH}");
    for method in [rtsp::RequestMethod::Options, rtsp::RequestMethod::Describe] {
        let request = rtsp::Request::new(method.clone(), stream_uri.as_str(), rtsp::Version::V1);
        match handle.send(request).await {
            Ok(response) => println!("{method} response:\n{response}"),
            Err(err) => {
                println!("{method} failed: {err}");
                break;
            }
        }
    }

    let reason = tokio::select! {
        reason = &mut task => reason?,
        _ = tokio::signal::ctrl_c() => {
            println!("Disconnecting");
            handle.close();
            task.await?
        }
    };
    println!("Connection closed: {reason}");

    Ok(())
}
