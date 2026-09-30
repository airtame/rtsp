use super::*;

#[test]
fn display_closed_by_peer() {
    assert_eq!(ConnectionCloseReason::ClosedByPeer.to_string(), "connection closed by peer");
}

#[test]
fn display_cancelled() {
    assert_eq!(ConnectionCloseReason::Cancelled.to_string(), "connection cancelled");
}

#[test]
fn display_idle_timeout() {
    assert_eq!(ConnectionCloseReason::IdleTimeout.to_string(), "connection idle timeout");
}

#[test]
fn display_io_includes_underlying_error() {
    let err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset by test");

    assert_eq!(ConnectionCloseReason::Io(err).to_string(), "connection I/O error: reset by test");
}
