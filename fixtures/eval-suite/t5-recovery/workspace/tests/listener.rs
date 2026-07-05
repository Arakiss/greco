use greco_t5_recovery_fixture::listen_address;

#[test]
fn listener_uses_normalized_port() {
    assert_eq!(listen_address(" 9000 "), "0.0.0.0:9000");
}

#[test]
fn listener_falls_back_to_default_port() {
    assert_eq!(listen_address("not-a-port"), "0.0.0.0:8080");
}
