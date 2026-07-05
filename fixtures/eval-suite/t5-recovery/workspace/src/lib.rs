pub mod port;

pub fn listen_address(raw_port: &str) -> String {
    format!("0.0.0.0:{}", normalize_port(raw_port))
}
