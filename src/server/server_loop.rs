use crate::connection::{Connection, ConnectionCloseReason, ConnectionOptions};
use crate::router::RequestHandler;
use crate::server::{ServerDelegate, ServerEvent};

pub(crate) struct ServerLoop {
    listener: tokio::net::TcpListener,
    cancellation_token: tokio_util::sync::CancellationToken,
    handler: std::sync::Arc<dyn RequestHandler>,
    delegate: std::sync::Arc<dyn ServerDelegate>,
    connection_options: ConnectionOptions,
    connection_tasks: tokio::task::JoinSet<(std::net::SocketAddr, ConnectionCloseReason)>,
}

impl ServerLoop {
    pub(crate) fn new(
        listener: tokio::net::TcpListener,
        cancellation_token: tokio_util::sync::CancellationToken,
        handler: std::sync::Arc<dyn RequestHandler>,
        delegate: std::sync::Arc<dyn ServerDelegate>,
        connection_options: ConnectionOptions,
    ) -> Self {
        Self {
            listener,
            cancellation_token,
            handler,
            delegate,
            connection_options,
            connection_tasks: tokio::task::JoinSet::new(),
        }
    }

    pub(crate) async fn run(mut self) {
        log::debug!("[rtsp] server run loop started");

        loop {
            let event = self.next_event().await;
            log::debug!("[rtsp] server event: {event:?}");

            match event {
                ServerEvent::Cancelled => break,
                ServerEvent::AcceptFailed(err) => {
                    log::error!("[rtsp] server failed to accept connection: {err}")
                }
                ServerEvent::ConnectionAccepted(stream, addr) => {
                    self.accept_connection(stream, addr)
                }
                ServerEvent::ConnectionClosed(closed_connection) => {
                    self.on_connection_closed(closed_connection)
                }
            }
        }

        while let Some(closed_connection) = self.connection_tasks.join_next().await {
            log::debug!("[rtsp] server stopping, connection closed: {closed_connection:?}");
            self.on_connection_closed(closed_connection);
        }

        log::debug!("[rtsp] server run loop stopped");
    }

    async fn next_event(&mut self) -> ServerEvent {
        tokio::select! {
            () = self.cancellation_token.cancelled() => ServerEvent::Cancelled,
            accepted = self.listener.accept() => match accepted {
                Ok((stream, addr)) => ServerEvent::ConnectionAccepted(stream, addr),
                Err(err) => ServerEvent::AcceptFailed(err),
            },
            Some(closed_connection) = self.connection_tasks.join_next() => {
                ServerEvent::ConnectionClosed(closed_connection)
            }
        }
    }

    fn accept_connection(&mut self, stream: tokio::net::TcpStream, addr: std::net::SocketAddr) {
        let (connection, connection_handle) = Connection::new(
            stream,
            addr,
            self.cancellation_token.child_token(),
            self.handler.clone(),
            self.connection_options.clone(),
        );
        log::debug!("[rtsp] new connection: {connection:?}");

        self.delegate.on_new_connection(connection_handle);

        self.connection_tasks
            .spawn(async move { (connection.peer_addr(), connection.run().await) });
    }

    fn on_connection_closed(
        &self,
        closed_connection: Result<
            (std::net::SocketAddr, ConnectionCloseReason),
            tokio::task::JoinError,
        >,
    ) {
        match closed_connection {
            Ok((addr, reason)) => self.delegate.on_connection_closed(addr, reason),
            Err(err) => log::error!("[rtsp] connection task failed: {err}"),
        }
    }
}

#[cfg(test)]
#[path = "test/server_loop_unittests.rs"]
mod tests;
