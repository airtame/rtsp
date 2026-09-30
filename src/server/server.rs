use crate::connection::{Connection, ConnectionCloseReason};

const MAX_CONNECTION_BACKLOG: u32 = 1024;

pub struct Server {
    listener: tokio::net::TcpListener,
    cancellation_token: tokio_util::sync::CancellationToken,
    connection_idle_timeout: Option<std::time::Duration>,
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
            connection_idle_timeout: None,
        })
    }

    pub fn with_connection_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.connection_idle_timeout = Some(timeout);
        self
    }

    pub async fn run(&self) {
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
                            self.accept_connection(&mut connection_tasks, stream, addr)
                        }
                        Err(err) => log::error!("[rtsp] server failed to accept connection: {err}"),
                    }
                }
                Some(closed_connection) = connection_tasks.join_next() => {
                    Self::on_connection_closed(closed_connection)
                }
            }
        }

        while let Some(closed_connection) = connection_tasks.join_next().await {
            Self::on_connection_closed(closed_connection);
        }

        log::debug!("[rtsp] server run loop stopped");
    }

    pub fn stop(&self) {
        log::debug!("[rtsp] server stop called");

        self.cancellation_token.cancel();
    }

    fn accept_connection(
        &self,
        connection_tasks: &mut tokio::task::JoinSet<(std::net::SocketAddr, ConnectionCloseReason)>,
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
    ) {
        log::debug!("[rtsp] new connection from {addr}");

        let mut connection = Connection::new(stream, addr, self.cancellation_token.child_token());
        if let Some(idle_timeout) = self.connection_idle_timeout {
            connection = connection.with_idle_timeout(idle_timeout);
        }

        connection_tasks.spawn(async move { (connection.peer_addr(), connection.run().await) });
    }

    fn on_connection_closed(
        closed_connection: Result<
            (std::net::SocketAddr, ConnectionCloseReason),
            tokio::task::JoinError,
        >,
    ) {
        match closed_connection {
            Ok((addr, reason)) => log::debug!("[rtsp] connection from {addr} closed: {reason}"),
            Err(err) => log::error!("[rtsp] connection task failed: {err}"),
        }
    }
}

#[cfg(test)]
#[path = "test/server_unittests.rs"]
mod tests;
