use super::*;

#[test]
fn display_connection_closed() {
    assert_eq!(
        RequestError::ConnectionClosed.to_string(),
        "connection closed before the response arrived"
    );
}
