use crate::connection::{Connection, ConnectionHandle, ConnectionOptions, ConnectionTask};
use crate::router::{RequestHandler, Router};

pub struct Client {
    handler: std::sync::Arc<dyn RequestHandler>,
    connection_options: ConnectionOptions,
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_handler(mut self, handler: impl RequestHandler + 'static) -> Self {
        self.handler = std::sync::Arc::new(handler);
        self
    }

    pub fn with_connection_options(mut self, options: ConnectionOptions) -> Self {
        self.connection_options = options;
        self
    }

    pub async fn connect(
        &self,
        addr: std::net::SocketAddr,
    ) -> std::io::Result<(ConnectionHandle, ConnectionTask)> {
        let stream = tokio::net::TcpStream::connect(addr).await?;

        log::debug!("[rtsp] client connected to {addr}");

        let peer_addr = stream.peer_addr()?;
        let (connection, handle) = Connection::new(
            stream,
            peer_addr,
            tokio_util::sync::CancellationToken::new(),
            self.handler.clone(),
            self.connection_options.clone(),
        );

        Ok((handle, ConnectionTask::new(connection)))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            handler: std::sync::Arc::new(Router::new()),
            connection_options: ConnectionOptions::default(),
        }
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("connection_options", &self.connection_options)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "test/client_unittests.rs"]
mod tests;
