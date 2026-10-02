use super::*;

#[test]
fn default_connection_options_are_default_options() {
    let peer_addr = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 8554));

    assert_eq!(().connection_options(peer_addr), ConnectionOptions::default());
}
