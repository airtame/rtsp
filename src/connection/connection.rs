use futures_util::{SinkExt, StreamExt};

use crate::connection::{
    ConnectionCloseReason, ConnectionHandle, ConnectionOptions, PendingRequest,
};
use crate::message::{Message, MessageCodec, MessageError, Response, StatusCode, Version};
use crate::router::RequestHandler;

const READ_BUFFER_SIZE: usize = 4096;
const CSEQ: &str = "CSeq";

pub(crate) struct Connection {
    framed_tcp_stream: tokio_util::codec::Framed<tokio::net::TcpStream, MessageCodec>,
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    handler: std::sync::Arc<dyn RequestHandler>,
    options: ConnectionOptions,
    request_rx: tokio::sync::mpsc::UnboundedReceiver<PendingRequest>,
    pending_responses: std::collections::HashMap<u32, tokio::sync::oneshot::Sender<Response>>,
    next_cseq: u32,
}

impl Connection {
    pub(crate) fn new(
        stream: tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
        handler: std::sync::Arc<dyn RequestHandler>,
        options: ConnectionOptions,
    ) -> (Self, ConnectionHandle) {
        let (request_tx, request_rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(peer_addr, cancellation_token.clone(), request_tx);
        let framed_tcp_stream = tokio_util::codec::Framed::with_capacity(
            stream,
            MessageCodec::new(options.parsing_mode()),
            READ_BUFFER_SIZE,
        );

        (
            Self {
                framed_tcp_stream,
                peer_addr,
                cancellation_token,
                handler,
                options,
                request_rx,
                pending_responses: std::collections::HashMap::new(),
                next_cseq: 1,
            },
            handle,
        )
    }

    pub(crate) fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub(crate) async fn run(mut self) -> ConnectionCloseReason {
        log::debug!("[rtsp] connection loop from {} started", self.peer_addr);

        let mut idle_timer =
            std::pin::pin!(tokio::time::sleep(self.options.idle_timeout().unwrap_or_default()));

        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    log::debug!("[rtsp] connection cancellation token triggered for {}", self.peer_addr);
                    return ConnectionCloseReason::Cancelled
                }
                _ = &mut idle_timer, if self.options.idle_timeout().is_some() => {
                    log::debug!("[rtsp] connection idle timeout triggered for {}", self.peer_addr);
                    return ConnectionCloseReason::IdleTimeout
                }
                message = self.framed_tcp_stream.next() => {
                    match message {
                        None => return ConnectionCloseReason::ClosedByPeer,
                        Some(Ok(decoded)) => {
                            if let Some(idle_timeout) = self.options.idle_timeout() {
                                idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            }
                            if let Some(activity_hook) = self.options.activity_hook() {
                                activity_hook(self.peer_addr);
                            }

                            match decoded {
                                Ok(Message::Request(request)) => {
                                    log::debug!("[rtsp] request from {}:\n{request}", self.peer_addr);

                                    let response = self.handler.handle(&request);
                                    let response = match request.headers().get(CSEQ) {
                                        Some(cseq) => response.with_cseq(cseq),
                                        None => response,
                                    };
                                    log::debug!("[rtsp] response to {}:\n{response}", self.peer_addr);

                                    if let Err(err) = self.framed_tcp_stream.send(Message::Response(response)).await {
                                        return ConnectionCloseReason::Io(err);
                                    }
                                }
                                Ok(Message::Response(response)) => {
                                    log::debug!("[rtsp] response from {}:\n{response}", self.peer_addr);

                                    let response_tx = response
                                        .headers()
                                        .get(CSEQ)
                                        .and_then(|cseq| cseq.parse::<u32>().ok())
                                        .and_then(|cseq| self.pending_responses.remove(&cseq));
                                    match response_tx {
                                        Some(response_tx) => {
                                            if response_tx.send(response).is_err() {
                                                log::debug!("[rtsp] response from {} arrived after its request was abandoned", self.peer_addr);
                                            }
                                        }
                                        None => log::warn!("[rtsp] response from {} matches no pending request", self.peer_addr),
                                    }
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
                Some(pending) = self.request_rx.recv() => {
                    let cseq = self.next_cseq;
                    self.next_cseq = self.next_cseq.wrapping_add(1);

                    let request = pending.request.with_cseq(cseq);
                    log::debug!("[rtsp] request to {}:\n{request}", self.peer_addr);

                    if let Err(err) = self.framed_tcp_stream.send(Message::Request(request)).await {
                        return ConnectionCloseReason::Io(err);
                    }
                    self.pending_responses.insert(cseq, pending.response_tx);
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

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("peer_addr", &self.peer_addr)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "test/connection_unittests.rs"]
mod tests;
