#[derive(Clone, Debug)]
pub struct ServerHandle {
    local_addr: std::net::SocketAddr,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl ServerHandle {
    pub(crate) fn new(
        local_addr: std::net::SocketAddr,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self { local_addr, cancellation_token }
    }

    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.local_addr
    }

    pub fn stop(&self) {
        log::debug!("[rtsp] server stop called");
        self.cancellation_token.cancel();
    }
}

#[cfg(test)]
#[path = "test/server_handle_unittests.rs"]
mod tests;
