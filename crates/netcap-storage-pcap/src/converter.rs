use netcap_core::capture::exchange::{CapturedExchange, CapturedRequest, CapturedResponse};
use std::fmt::Write as FmtWrite;

/// Build a pseudo HTTP request as raw bytes (request line + headers + body)
fn build_http_request_payload(req: &CapturedRequest) -> Vec<u8> {
    let mut payload = String::new();
    let path = req.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    let version = match req.version {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2",
        http::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    };
    let _ = writeln!(payload, "{} {} {}\r", req.method, path, version);
    for (name, value) in req.headers.iter() {
        if let Ok(v) = value.to_str() {
            let _ = writeln!(payload, "{}: {}\r", name, v);
        }
    }
    let _ = write!(payload, "\r\n");
    let mut bytes = payload.into_bytes();
    if !req.body.is_empty() {
        bytes.extend_from_slice(&req.body);
    }
    bytes
}

/// Build a pseudo HTTP response as raw bytes (status line + headers + body)
fn build_http_response_payload(resp: &CapturedResponse) -> Vec<u8> {
    let mut payload = String::new();
    let version = match resp.version {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2",
        http::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    };
    let _ = writeln!(
        payload,
        "{} {} {}\r",
        version,
        resp.status.as_u16(),
        resp.status.canonical_reason().unwrap_or("")
    );
    for (name, value) in resp.headers.iter() {
        if let Ok(v) = value.to_str() {
            let _ = writeln!(payload, "{}: {}\r", name, v);
        }
    }
    let _ = write!(payload, "\r\n");
    let mut bytes = payload.into_bytes();
    if !resp.body.is_empty() {
        bytes.extend_from_slice(&resp.body);
    }
    bytes
}

/// Wrap an HTTP payload in pseudo Ethernet + IPv4 + TCP headers.
/// Uses dummy addresses: client=127.0.0.1:50000 → server=127.0.0.2:80/443
fn wrap_in_tcp_ip(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let ip_total_len = (20 + 20 + payload.len()) as u16;
    let mut pkt = Vec::with_capacity(14 + 20 + 20 + payload.len());

    // Ethernet header (14 bytes)
    pkt.extend_from_slice(&[0u8; 6]); // dst mac
    pkt.extend_from_slice(&[0u8; 6]); // src mac
    pkt.extend_from_slice(&[0x08, 0x00]); // EtherType: IPv4

    // IPv4 header (20 bytes)
    pkt.push(0x45); // version=4, IHL=5
    pkt.push(0x00); // DSCP/ECN
    pkt.extend_from_slice(&ip_total_len.to_be_bytes());
    pkt.extend_from_slice(&[0x00, 0x00]); // identification
    pkt.extend_from_slice(&[0x40, 0x00]); // flags=DF, fragment offset=0
    pkt.push(64); // TTL
    pkt.push(6); // protocol=TCP
    pkt.extend_from_slice(&[0x00, 0x00]); // header checksum (0 = not computed)
    pkt.extend_from_slice(&src_ip);
    pkt.extend_from_slice(&dst_ip);

    // TCP header (20 bytes, simplified)
    pkt.extend_from_slice(&src_port.to_be_bytes());
    pkt.extend_from_slice(&dst_port.to_be_bytes());
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // seq number
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // ack number
    pkt.push(0x50); // data offset = 5 (20 bytes)
    pkt.push(0x18); // flags: PSH + ACK
    pkt.extend_from_slice(&[0xFF, 0xFF]); // window size
    pkt.extend_from_slice(&[0x00, 0x00]); // checksum (0)
    pkt.extend_from_slice(&[0x00, 0x00]); // urgent pointer

    // Payload
    pkt.extend_from_slice(payload);
    pkt
}

/// Build a request packet for the given exchange.
pub fn build_request_packet(exchange: &CapturedExchange) -> Vec<u8> {
    let payload = build_http_request_payload(&exchange.request);
    let is_tls = exchange.request.tls_info.is_some();
    let dst_port = if is_tls { 443 } else { 80 };
    wrap_in_tcp_ip(
        [127, 0, 0, 1], // client
        [127, 0, 0, 2], // server
        50000,
        dst_port,
        &payload,
    )
}

/// Build a response packet for the given exchange (if response exists).
pub fn build_response_packet(exchange: &CapturedExchange) -> Option<Vec<u8>> {
    let resp = exchange.response.as_ref()?;
    let payload = build_http_response_payload(resp);
    let is_tls = exchange.request.tls_info.is_some();
    let src_port = if is_tls { 443 } else { 80 };
    Some(wrap_in_tcp_ip(
        [127, 0, 0, 2], // server
        [127, 0, 0, 1], // client
        src_port,
        50000,
        &payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use uuid::Uuid;

    fn make_request() -> CapturedRequest {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        CapturedRequest {
            id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            connection_id: Uuid::now_v7(),
            sequence_number: 0,
            timestamp: Utc::now(),
            method: Method::GET,
            uri: "http://example.com/path?q=1".parse().unwrap(),
            version: Version::HTTP_11,
            headers,
            body: Bytes::from("request body"),
            body_truncated: false,
            tls_info: None,
        }
    }

    fn make_response() -> CapturedResponse {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/plain".parse().unwrap());
        CapturedResponse {
            id: Uuid::now_v7(),
            request_id: Uuid::now_v7(),
            timestamp: Utc::now(),
            status: StatusCode::OK,
            version: Version::HTTP_11,
            headers,
            body: Bytes::from("response body"),
            body_truncated: false,
            latency: std::time::Duration::from_millis(50),
            ttfb: std::time::Duration::from_millis(10),
        }
    }

    #[test]
    fn request_packet_has_ethernet_ip_tcp_headers() {
        let exchange = CapturedExchange {
            request: make_request(),
            response: None,
        };
        let pkt = build_request_packet(&exchange);
        // Ethernet (14) + IP (20) + TCP (20) + payload
        assert!(pkt.len() > 54);
        // EtherType = 0x0800 (IPv4)
        assert_eq!(pkt[12], 0x08);
        assert_eq!(pkt[13], 0x00);
        // IP version = 4, IHL = 5
        assert_eq!(pkt[14], 0x45);
        // Protocol = TCP (6)
        assert_eq!(pkt[23], 6);
        // Dst port = 80 (no TLS)
        assert_eq!(u16::from_be_bytes([pkt[36], pkt[37]]), 80);
    }

    #[test]
    fn request_packet_tls_uses_port_443() {
        let mut req = make_request();
        req.tls_info = Some(netcap_core::capture::exchange::TlsInfo {
            sni: "example.com".into(),
            protocol_version: "TLSv1.3".into(),
            cipher_suite: "AES256".into(),
        });
        let exchange = CapturedExchange {
            request: req,
            response: None,
        };
        let pkt = build_request_packet(&exchange);
        assert_eq!(u16::from_be_bytes([pkt[36], pkt[37]]), 443);
    }

    #[test]
    fn response_packet_present_when_response_exists() {
        let exchange = CapturedExchange {
            request: make_request(),
            response: Some(make_response()),
        };
        let pkt = build_response_packet(&exchange);
        assert!(pkt.is_some());
        let pkt = pkt.unwrap();
        assert!(pkt.len() > 54);
        // Src port = 80 (server)
        assert_eq!(u16::from_be_bytes([pkt[34], pkt[35]]), 80);
    }

    #[test]
    fn response_packet_none_when_no_response() {
        let exchange = CapturedExchange {
            request: make_request(),
            response: None,
        };
        assert!(build_response_packet(&exchange).is_none());
    }

    #[test]
    fn payload_contains_http_request_line() {
        let exchange = CapturedExchange {
            request: make_request(),
            response: None,
        };
        let pkt = build_request_packet(&exchange);
        let payload = &pkt[54..]; // after Eth+IP+TCP
        let text = String::from_utf8_lossy(payload);
        assert!(text.contains("GET /path?q=1 HTTP/1.1"));
        assert!(text.contains("host: example.com"));
        assert!(text.contains("request body"));
    }

    #[test]
    fn payload_contains_http_status_line() {
        let exchange = CapturedExchange {
            request: make_request(),
            response: Some(make_response()),
        };
        let pkt = build_response_packet(&exchange).unwrap();
        let payload = &pkt[54..];
        let text = String::from_utf8_lossy(payload);
        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(text.contains("content-type: text/plain"));
        assert!(text.contains("response body"));
    }
}
