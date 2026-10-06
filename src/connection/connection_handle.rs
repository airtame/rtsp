use crate::connection::{PendingRequest, ResponseFuture};
use crate::message::Request;

#[derive(Clone)]
pub struct ConnectionHandle {
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
    request_tx: tokio::sync::mpsc::UnboundedSender<PendingRequest>,
}

impl ConnectionHandle {
    pub(crate) fn new(
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
        request_tx: tokio::sync::mpsc::UnboundedSender<PendingRequest>,
    ) -> Self {
        Self { peer_addr, cancellation_token, request_tx }
    }

    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub fn close(&self) {
        log::debug!("[rtsp] connection close requested by embedder for {}", self.peer_addr);
        self.cancellation_token.cancel();
    }

    pub fn is_closed(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub fn send(&self, request: Request) -> ResponseFuture {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let _ = self.request_tx.send(PendingRequest { request, response_tx });

        ResponseFuture::new(response_rx)
    }
}

impl std::fmt::Debug for ConnectionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionHandle")
            .field("peer_addr", &self.peer_addr)
            .field("is_closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "test/connection_handle_unittests.rs"]
mod tests;
