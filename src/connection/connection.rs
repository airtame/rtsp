use futures_util::StreamExt;

use crate::connection::{ConnectionCloseReason, ConnectionHandle};
use crate::message::{MessageCodec, MessageError};

const READ_BUFFER_SIZE: usize = 4096;

pub(crate) struct Connection {
    framed_tcp_stream: tokio_util::codec::Framed<tokio::net::TcpStream, MessageCodec>,
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    idle_timeout: Option<std::time::Duration>,
}

impl Connection {
    pub(crate) fn new(
        stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> (Self, ConnectionHandle) {
        let handle = ConnectionHandle::new(peer_addr, cancellation_token.clone());
        let framed_tcp_stream = tokio_util::codec::Framed::with_capacity(
            stream,
            MessageCodec::default(),
            READ_BUFFER_SIZE,
        );

        (Self { framed_tcp_stream, peer_addr, cancellation_token, idle_timeout: None }, handle)
    }

    pub(crate) fn with_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    pub(crate) fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub(crate) async fn run(mut self) -> ConnectionCloseReason {
        log::debug!("[rtsp] connection loop from {} started", self.peer_addr);

        let mut idle_timer =
            std::pin::pin!(tokio::time::sleep(self.idle_timeout.unwrap_or_default()));

        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    log::debug!("[rtsp] connection cancellation token triggered for {}", self.peer_addr);
                    return ConnectionCloseReason::Cancelled
                }
                _ = &mut idle_timer, if self.idle_timeout.is_some() => {
                    log::debug!("[rtsp] connection idle timeout triggered for {}", self.peer_addr);
                    return ConnectionCloseReason::IdleTimeout
                }
                message = self.framed_tcp_stream.next() => {
                    match message {
                        None => return ConnectionCloseReason::ClosedByPeer,
                        // NOTE(atokodi): Messages are only logged until request handling is implemented.
                        Some(Ok(message)) => {
                            log::debug!("[rtsp] message from {}:\n{message}", self.peer_addr);
                            if let Some(idle_timeout) = self.idle_timeout {
                                idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            }
                        }
                        Some(Err(MessageError::Io(err))) => return ConnectionCloseReason::Io(err),
                        Some(Err(err)) => return ConnectionCloseReason::InvalidMessage(err),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "test/connection_unittests.rs"]
mod tests;
