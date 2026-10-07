use futures_util::{SinkExt, StreamExt};

use crate::connection::{
    ConnectionCloseReason, ConnectionEvent, ConnectionHandle, ConnectionOptions, PendingRequest,
};
use crate::message::{
    CSeqHeader, Message, MessageCodec, MessageError, Request, Response, StatusCode, Version,
};
use crate::router::RequestHandler;

const READ_BUFFER_SIZE: usize = 4096;

pub(crate) struct Connection {
    framed_tcp_stream: tokio_util::codec::Framed<tokio::net::TcpStream, MessageCodec>,
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    handler: std::sync::Arc<dyn RequestHandler>,
    options: ConnectionOptions,
    idle_timer: std::pin::Pin<Box<tokio::time::Sleep>>,
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
        let idle_timer = Box::pin(tokio::time::sleep(options.idle_timeout().unwrap_or_default()));

        (
            Self {
                framed_tcp_stream,
                peer_addr,
                cancellation_token,
                handler,
                options,
                idle_timer,
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

        self.reset_idle_timer();

        loop {
            let event = self.next_event().await;
            log::debug!("[rtsp] connection event from {}: {event:?}", self.peer_addr);

            match event {
                ConnectionEvent::Cancelled => return ConnectionCloseReason::Cancelled,
                ConnectionEvent::IdleTimeout => return ConnectionCloseReason::IdleTimeout,
                ConnectionEvent::ClosedByPeer => return ConnectionCloseReason::ClosedByPeer,
                ConnectionEvent::ReadFailed(err) => return ConnectionCloseReason::Io(err),
                ConnectionEvent::InvalidMessage(err) => {
                    if let Err(send_err) = self.reject(&err, None).await {
                        return ConnectionCloseReason::Io(send_err);
                    }
                    return ConnectionCloseReason::InvalidMessage(err);
                }
                ConnectionEvent::MalformedMessage(malformed) => {
                    self.record_activity();
                    if let Err(err) = self.reject(&malformed.error, malformed.cseq).await {
                        return ConnectionCloseReason::Io(err);
                    }
                }
                ConnectionEvent::RequestReceived(request) => {
                    self.record_activity();
                    if let Err(err) = self.on_request_received(request).await {
                        return ConnectionCloseReason::Io(err);
                    }
                }
                ConnectionEvent::ResponseReceived(response) => {
                    self.record_activity();
                    self.on_response_received(response);
                }
                ConnectionEvent::RequestQueued(pending) => {
                    if let Err(err) = self.on_request_queued(pending).await {
                        return ConnectionCloseReason::Io(err);
                    }
                }
            }
        }
    }

    async fn next_event(&mut self) -> ConnectionEvent {
        tokio::select! {
            () = self.cancellation_token.cancelled() => ConnectionEvent::Cancelled,
            () = &mut self.idle_timer, if self.options.idle_timeout().is_some() => {
                ConnectionEvent::IdleTimeout
            }
            message = self.framed_tcp_stream.next() => match message {
                None => ConnectionEvent::ClosedByPeer,
                Some(Ok(Ok(Message::Request(request)))) => {
                    ConnectionEvent::RequestReceived(request)
                }
                Some(Ok(Ok(Message::Response(response)))) => {
                    ConnectionEvent::ResponseReceived(response)
                }
                Some(Ok(Err(malformed))) => ConnectionEvent::MalformedMessage(malformed),
                Some(Err(MessageError::Io(err))) => ConnectionEvent::ReadFailed(err),
                Some(Err(err)) => ConnectionEvent::InvalidMessage(err),
            },
            Some(pending) = self.request_rx.recv() => ConnectionEvent::RequestQueued(pending),
        }
    }

    async fn on_request_received(&mut self, request: Request) -> std::io::Result<()> {
        let response = self.handler.handle(&request);
        let response = match request.headers().typed::<CSeqHeader>() {
            Some(Ok(cseq)) => response.with_typed_header(cseq),
            _ => response,
        };
        log::debug!("[rtsp] response to {}:\n{response}", self.peer_addr);

        self.framed_tcp_stream.send(Message::Response(response)).await
    }

    fn on_response_received(&mut self, response: Response) {
        let response_tx = response
            .headers()
            .typed::<CSeqHeader>()
            .and_then(Result::ok)
            .and_then(|CSeqHeader(cseq)| self.pending_responses.remove(&cseq));

        match response_tx {
            Some(response_tx) => {
                if response_tx.send(response).is_err() {
                    log::debug!(
                        "[rtsp] response from {} arrived after its request was abandoned",
                        self.peer_addr
                    );
                }
            }
            None => {
                log::warn!("[rtsp] response from {} matches no pending request", self.peer_addr)
            }
        }
    }

    async fn on_request_queued(&mut self, pending: PendingRequest) -> std::io::Result<()> {
        let cseq = self.next_cseq;
        self.next_cseq = self.next_cseq.wrapping_add(1);

        let request = pending.request.with_typed_header(CSeqHeader(cseq));
        log::debug!("[rtsp] request to {}:\n{request}", self.peer_addr);

        self.framed_tcp_stream.send(Message::Request(request)).await?;
        self.pending_responses.insert(cseq, pending.response_tx);

        Ok(())
    }

    fn record_activity(&mut self) {
        self.reset_idle_timer();
        if let Some(activity_hook) = self.options.activity_hook() {
            activity_hook(self.peer_addr);
        }
    }

    fn reset_idle_timer(&mut self) {
        if let Some(idle_timeout) = self.options.idle_timeout() {
            self.idle_timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
        }
    }

    async fn reject(
        &mut self,
        error: &MessageError,
        cseq: Option<CSeqHeader>,
    ) -> std::io::Result<()> {
        log::error!("[rtsp] rejecting message from {}: {error}", self.peer_addr);

        let mut response = Response::new(Version::V1, StatusCode::BadRequest);
        if let Some(cseq) = cseq {
            response = response.with_typed_header(cseq);
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
