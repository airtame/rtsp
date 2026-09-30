const MAX_CONNECTION_BACKLOG: u32 = 1024;

pub struct Server {
    listener: tokio::net::TcpListener,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl Server {
    pub fn bind(addr: std::net::SocketAddr) -> std::io::Result<Self> {
        let socket = match addr {
            std::net::SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
            std::net::SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
        }?;

        socket.set_reuseaddr(true)?;
        socket.bind(addr)?;

        let listener = socket.listen(MAX_CONNECTION_BACKLOG)?;

        log::debug!("[rtsp] server bind successful to {addr}");

        Ok(Self { listener, cancellation_token: tokio_util::sync::CancellationToken::new() })
    }

    pub async fn run(&self) {
        log::debug!("[rtsp] server run loop started");

        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    log::debug!("[rtsp] server cancellation token triggered");
                    break;
                }
                accepted = self.listener.accept() => {
                    match accepted {
                        Ok((stream, addr)) => {
                            self.accept_connection(stream, addr)
                        }
                        Err(err) => log::error!("[rtsp] server failed to accept connection: {err}"),
                    }
                }
            }
        }
    }

    pub fn stop(&self) {
        log::debug!("[rtsp] server stop called");
        self.cancellation_token.cancel();
    }

    fn accept_connection(&self, _stream: tokio::net::TcpStream, addr: std::net::SocketAddr) {
        log::debug!("[rtsp] new connection from {addr}");
    }
}

#[cfg(test)]
#[path = "test/server_unittests.rs"]
mod tests;
