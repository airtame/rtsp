use crate::connection::{ConnectionCloseReason, ConnectionHandle};

// TODO(atokodi): Later when adding rust docs to the project don't forget to mention that these
// functions should not block since they are running in the server mail loop
pub trait ServerDelegate {
    fn on_new_connection(&self, _connection: ConnectionHandle) {}

    fn on_connection_closed(
        &self,
        _peer_addr: std::net::SocketAddr,
        _reason: ConnectionCloseReason,
    ) {
    }
}

impl ServerDelegate for () {}
