use crate::connection::ConnectionOptions;
use crate::router::{RequestHandler, Router};
use crate::server::{ServerDelegate, ServerHandle, ServerLoop, ServerTask};

const MAX_CONNECTION_BACKLOG: u32 = 1024;

pub struct Server {
    handler: std::sync::Arc<dyn RequestHandler>,
    delegate: std::sync::Arc<dyn ServerDelegate>,
    connection_options: ConnectionOptions,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_handler(mut self, handler: impl RequestHandler + 'static) -> Self {
        self.handler = std::sync::Arc::new(handler);
        self
    }

    pub fn with_delegate(mut self, delegate: impl ServerDelegate + 'static) -> Self {
        self.delegate = std::sync::Arc::new(delegate);
        self
    }

    pub fn with_connection_options(mut self, options: ConnectionOptions) -> Self {
        self.connection_options = options;
        self
    }

    pub fn bind(&self, addr: std::net::SocketAddr) -> std::io::Result<(ServerHandle, ServerTask)> {
        let socket = match addr {
            std::net::SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
            std::net::SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
        }?;

        socket.set_reuseaddr(true)?;
        socket.bind(addr)?;

        let listener = socket.listen(MAX_CONNECTION_BACKLOG)?;
        let local_addr = listener.local_addr()?;

        log::debug!("[rtsp] server bind successful to {local_addr}");

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let handle = ServerHandle::new(local_addr, cancellation_token.clone());
        let server_loop = ServerLoop::new(
            listener,
            cancellation_token,
            self.handler.clone(),
            self.delegate.clone(),
            self.connection_options.clone(),
        );

        Ok((handle, ServerTask::new(server_loop)))
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            handler: std::sync::Arc::new(Router::new()),
            delegate: std::sync::Arc::new(()),
            connection_options: ConnectionOptions::default(),
        }
    }
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("connection_options", &self.connection_options)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "test/server_unittests.rs"]
mod tests;
