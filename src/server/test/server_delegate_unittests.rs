use super::*;

#[test]
fn default_connection_options_are_default_options() {
    let peer_addr = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 8554));

    assert_eq!(
        format!("{:?}", ().connection_options(peer_addr)),
        format!("{:?}", ConnectionOptions::default())
    );
}
