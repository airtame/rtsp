#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    peer_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl ConnectionHandle {
    pub(crate) fn new(
        peer_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self { peer_addr, cancellation_token }
    }

    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub fn close(&self) {
        log::debug!("[rtsp] connection close requested by embedder for {}", self.peer_addr);
        self.cancellation_token.cancel();
    }
}

#[cfg(test)]
#[path = "test/connection_handle_unittests.rs"]
mod tests;
