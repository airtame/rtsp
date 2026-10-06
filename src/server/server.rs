use crate::connection::{Connection, ConnectionCloseReason, ConnectionOptions};
use crate::router::{RequestHandler, Router};
use crate::server::ServerDelegate;

const MAX_CONNECTION_BACKLOG: u32 = 1024;

pub struct Server {
    listener: tokio::net::TcpListener,
    cancellation_token: tokio_util::sync::CancellationToken,
    handler: std::sync::Arc<dyn RequestHandler>,
    connection_options: ConnectionOptions,
}

impl Server {
    pub fn bind(addr: std::net::SocketAddr) -> std::io::Result<Self> {
        let socket = match addr {
            std::net::SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
            std::net::SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
        }?;

        socket.set_reuseaddr(true)?;
        socket.bind(addr)?;

        let listener = socket.listen(MAX_CONNECTION_BACKLOG)?;

        log::debug!("[rtsp] server bind successful to {addr}");

        Ok(Self {
            listener,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            handler: std::sync::Arc::new(Router::new()),
            connection_options: ConnectionOptions::default(),
        })
    }

    pub fn with_handler(mut self, handler: impl RequestHandler + 'static) -> Self {
        self.handler = std::sync::Arc::new(handler);
        self
    }

    pub fn with_connection_options(mut self, options: ConnectionOptions) -> Self {
        self.connection_options = options;
        self
    }

    pub async fn run<D: ServerDelegate>(&self, delegate: &D) {
        log::debug!("[rtsp] server run loop started");

        let mut connection_tasks = tokio::task::JoinSet::new();

        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    log::debug!("[rtsp] server cancellation token triggered");
                    break;
                }
                accepted = self.listener.accept() => {
                    match accepted {
                        Ok((stream, addr)) => {
                            self.accept_connection(&mut connection_tasks, stream, addr, delegate)
                        }
                        Err(err) => log::error!("[rtsp] server failed to accept connection: {err}"),
                    }
                }
                Some(closed_connection) = connection_tasks.join_next() => {
                    Self::on_connection_closed(closed_connection, delegate)
                }
            }
        }

        while let Some(closed_connection) = connection_tasks.join_next().await {
            Self::on_connection_closed(closed_connection, delegate);
        }

        log::debug!("[rtsp] server run loop stopped");
    }

    pub fn stop(&self) {
        log::debug!("[rtsp] server stop called");

        self.cancellation_token.cancel();
    }

    fn accept_connection<D: ServerDelegate>(
        &self,
        connection_tasks: &mut tokio::task::JoinSet<(std::net::SocketAddr, ConnectionCloseReason)>,
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        delegate: &D,
    ) {
        let (connection, connection_handle) = Connection::new(
            stream,
            addr,
            self.cancellation_token.child_token(),
            self.handler.clone(),
            self.connection_options.clone(),
        );
        log::debug!("[rtsp] new connection: {connection:?}");

        delegate.on_new_connection(connection_handle);

        connection_tasks.spawn(async move { (connection.peer_addr(), connection.run().await) });
    }

    fn on_connection_closed<D: ServerDelegate>(
        closed_connection: Result<
            (std::net::SocketAddr, ConnectionCloseReason),
            tokio::task::JoinError,
        >,
        delegate: &D,
    ) {
        match closed_connection {
            Ok((addr, reason)) => {
                log::debug!("[rtsp] connection from {addr} closed: {reason}");
                delegate.on_connection_closed(addr, reason);
            }
            Err(err) => log::error!("[rtsp] connection task failed: {err}"),
        }
    }
}

#[cfg(test)]
#[path = "test/server_unittests.rs"]
mod tests;
