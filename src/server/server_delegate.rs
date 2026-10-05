use crate::connection::{ConnectionCloseReason, ConnectionHandle, ConnectionOptions};

// TODO(atokodi): Later when adding rust docs to the project don't forget to mention that these
// functions should not block since they are running in the server main loop
pub trait ServerDelegate {
    fn connection_options(&self, _peer_addr: std::net::SocketAddr) -> ConnectionOptions {
        ConnectionOptions::default()
    }

    fn on_new_connection(&self, _connection: ConnectionHandle) {}

    fn on_connection_closed(
        &self,
        _peer_addr: std::net::SocketAddr,
        _reason: ConnectionCloseReason,
    ) {
    }
}

impl ServerDelegate for () {}

#[cfg(test)]
#[path = "test/server_delegate_unittests.rs"]
mod tests;
