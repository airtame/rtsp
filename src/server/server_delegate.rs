use crate::connection::{ConnectionCloseReason, ConnectionHandle};

pub trait ServerDelegate: Send + Sync {
    fn on_new_connection(&self, _connection: ConnectionHandle) {}

    fn on_connection_closed(
        &self,
        _peer_addr: std::net::SocketAddr,
        _reason: ConnectionCloseReason,
    ) {
    }
}

impl ServerDelegate for () {}
