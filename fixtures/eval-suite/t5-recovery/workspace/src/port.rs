pub fn normalize(raw_port: &str) -> u16 {
    raw_port.trim().parse::<u16>().unwrap_or(8080)
}
