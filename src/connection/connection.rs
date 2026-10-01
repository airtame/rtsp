use futures_util::{SinkExt, StreamExt};

use crate::connection::{ConnectionCloseReason, ConnectionHandle};
use crate::message::{Message, MessageCodec, MessageError, Response, StatusCode, Version};
use crate::router::Router;

const READ_BUFFER_SIZE: usize = 4096;

pub(crate) struct Connection {
    framed_tcp_stream: tokio_util::codec::Framed<tokio::net::TcpStream, MessageCodec>,
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    idle_timeout: Option<std::time::Duration>,
    router: Router,
}

impl Connection {
    pub(crate) fn new(
        stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
        router: Router,
    ) -> (Self, ConnectionHandle) {
        let handle = ConnectionHandle::new(peer_addr, cancellation_token.clone());
        let framed_tcp_stream = tokio_util::codec::Framed::with_capacity(
            stream,
            MessageCodec::default(),
            READ_BUFFER_SIZE,
        );

        (
            Self { framed_tcp_stream, peer_addr, cancellation_token, idle_timeout: None, router },
            handle,
        )
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
                        Some(Ok(decoded)) => {
                            if let Some(idle_timeout) = self.idle_timeout {
                                idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            }

                            match decoded {
                                Ok(Message::Request(request)) => {
                                    log::debug!("[rtsp] request from {}:\n{request}", self.peer_addr);

                                    let response = self.router.route(&request);
                                    log::debug!("[rtsp] response to {}:\n{response}", self.peer_addr);

                                    if let Err(err) = self.framed_tcp_stream.send(Message::Response(response)).await {
                                        return ConnectionCloseReason::Io(err);
                                    }
                                }
                                Ok(Message::Response(response)) => {
                                    log::debug!("[rtsp] response from {}:\n{response}", self.peer_addr);
                                    // TODO(atokodi): Should notify the embedder about this new
                                    // message so it can process it.
                                }
                                Err(malformed) => {
                                    if let Err(err) = self.reject(&malformed.error, malformed.cseq.as_deref()).await {
                                        return ConnectionCloseReason::Io(err);
                                    }
                                }
                            }
                        }
                        Some(Err(MessageError::Io(err))) => return ConnectionCloseReason::Io(err),
                        Some(Err(err)) => {
                            if let Err(send_err) = self.reject(&err, None).await {
                                return ConnectionCloseReason::Io(send_err);
                            }
                            return ConnectionCloseReason::InvalidMessage(err);
                        }
                    }
                }
            }
        }
    }

    async fn reject(&mut self, error: &MessageError, cseq: Option<&str>) -> std::io::Result<()> {
        log::error!("[rtsp] rejecting message from {}: {error}", self.peer_addr);

        let mut response = Response::new(Version::V1, StatusCode::BadRequest);
        if let Some(cseq) = cseq {
            response = response.with_cseq(cseq);
        }

        self.framed_tcp_stream.send(Message::Response(response)).await
    }
}

#[cfg(test)]
#[path = "test/connection_unittests.rs"]
mod tests;
