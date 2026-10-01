use crate::message::MessageError;

#[derive(Debug)]
pub enum ConnectionCloseReason {
    ClosedByPeer,
    Cancelled,
    Io(std::io::Error),
    IdleTimeout,
    InvalidMessage(MessageError),
}

impl std::fmt::Display for ConnectionCloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClosedByPeer => write!(f, "connection closed by peer"),
            Self::Cancelled => write!(f, "connection cancelled"),
            Self::Io(err) => write!(f, "connection I/O error: {err}"),
            Self::IdleTimeout => write!(f, "connection idle timeout"),
            Self::InvalidMessage(err) => write!(f, "invalid message: {err}"),
        }
    }
}

#[cfg(test)]
#[path = "test/close_reason_unittests.rs"]
mod tests;
