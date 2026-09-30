use tokio::io::AsyncReadExt;

use crate::connection::ConnectionCloseReason;

const READ_BUFFER_SIZE: usize = 4096;

pub struct Connection {
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    idle_timeout: Option<std::time::Duration>,
}

impl Connection {
    pub fn new(
        stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self { stream, peer_addr, cancellation_token, idle_timeout: None }
    }

    pub fn with_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub async fn run(mut self) -> ConnectionCloseReason {
        log::debug!("[rtsp] connection loop from {} started", self.peer_addr);

        let mut idle_timer =
            std::pin::pin!(tokio::time::sleep(self.idle_timeout.unwrap_or_default()));

        let mut buffer = [0u8; READ_BUFFER_SIZE];
        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    log::debug!("[rtsp] client cancellation token triggered for {}", self.peer_addr);
                    return ConnectionCloseReason::Cancelled
                }
                _ = &mut idle_timer, if self.idle_timeout.is_some() => {
                    return ConnectionCloseReason::IdleTimeout
                }
                read = self.stream.read(&mut buffer) => {
                    match read {
                        Ok(0) => return ConnectionCloseReason::ClosedByPeer,
                        // NOTE(atokodi): Incoming data is discarded until request framing is implemented.
                        Ok(_) => {
                            if let Some(idle_timeout) = self.idle_timeout {
                                idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            }
                        }
                        Err(err) => return ConnectionCloseReason::Io(err),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "test/connection_unittests.rs"]
mod tests;
