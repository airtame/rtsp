use crate::connection::ConnectionCloseReason;

#[derive(Debug)]
pub(crate) enum ServerEvent {
    Cancelled,
    AcceptFailed(std::io::Error),
    ConnectionAccepted(tokio::net::TcpStream, std::net::SocketAddr),
    ConnectionClosed(Result<(std::net::SocketAddr, ConnectionCloseReason), tokio::task::JoinError>),
}
