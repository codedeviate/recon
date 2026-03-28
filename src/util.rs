/// Extracts (host, port) from any of:
///   "example.com", "example.com:8080", "https://example.com", "https://example.com:8080"
pub fn parse_target(input: &str) -> (String, Option<u16>) {
    // Strip protocol
    let s = if let Some(pos) = input.find("://") {
        &input[pos + 3..]
    } else {
        input
    };
    // Strip path, query, fragment
    let s = s
        .split(|c| c == '/' || c == '?' || c == '#')
        .next()
        .unwrap_or(s);
    // IPv6 [::1]:port
    if s.starts_with('[') {
        if let Some(end) = s.find(']') {
            let host = s[1..end].to_string();
            let port = s[end + 1..]
                .strip_prefix(':')
                .and_then(|p| p.parse().ok());
            return (host, port);
        }
    }
    // Hostname or IPv4 with optional port
    if let Some(pos) = s.rfind(':') {
        if let Ok(port) = s[pos + 1..].parse::<u16>() {
            return (s[..pos].to_string(), Some(port));
        }
    }
    (s.to_string(), None)
}
