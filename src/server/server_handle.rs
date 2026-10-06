#[derive(Clone, Debug)]
pub struct ServerHandle {
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl ServerHandle {
    pub(crate) fn new(cancellation_token: tokio_util::sync::CancellationToken) -> Self {
        Self { cancellation_token }
    }

    pub fn stop(&self) {
        log::debug!("[rtsp] server stop called");
        self.cancellation_token.cancel();
    }
}

#[cfg(test)]
#[path = "test/server_handle_unittests.rs"]
mod tests;
