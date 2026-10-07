use crate::models::ProbeStatus;
use rand::Rng;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{lookup_host, TcpStream, UdpSocket};
use tokio::time::timeout;

const DNS_TIMEOUT: Duration = Duration::from_secs(3);

pub struct DnsProbeOutcome {
    pub status: ProbeStatus,
    pub latency_ms: u128,
    pub detail: String,
    pub reason_code: Option<String>,
}

pub async fn probe(resolver: &str, host: &str) -> DnsProbeOutcome {
    let started = Instant::now();
    if resolver.eq_ignore_ascii_case("SYSTEM") {
        return match timeout(DNS_TIMEOUT, lookup_host((host, 443))).await {
            Ok(Ok(addresses)) => {
                let values: Vec<String> =
                    addresses.map(|address| address.ip().to_string()).collect();
                if values.is_empty() {
                    failed(
                        started,
                        "System resolver returned no addresses",
                        "dns_empty",
                    )
                } else {
                    passed(started, format!("System DNS: {}", values.join(", ")))
                }
            }
            Ok(Err(error)) => failed(started, error.to_string(), "dns_system_error"),
            Err(_) => failed(started, "System DNS timeout", "dns_timeout"),
        };
    }

    let resolver_ip = match resolver.parse::<IpAddr>() {
        Ok(value) => value,
        Err(_) => {
            return failed(
                started,
                "Invalid DNS resolver address",
                "dns_invalid_resolver",
            )
        }
    };
    let (a, aaaa) = tokio::join!(
        probe_direct(resolver_ip, host, 1, "A"),
        probe_direct(resolver_ip, host, 28, "AAAA")
    );
    let status = if a.status == ProbeStatus::Passed || aaaa.status == ProbeStatus::Passed {
        ProbeStatus::Passed
    } else if a.status == ProbeStatus::Inconclusive || aaaa.status == ProbeStatus::Inconclusive {
        ProbeStatus::Inconclusive
    } else {
        ProbeStatus::Failed
    };
    DnsProbeOutcome {
        status,
        latency_ms: started.elapsed().as_millis(),
        detail: format!("{}; {}", a.detail, aaaa.detail),
        reason_code: if status == ProbeStatus::Passed {
            None
        } else {
            a.reason_code.or(aaaa.reason_code)
        },
    }
}

async fn probe_direct(
    resolver_ip: IpAddr,
    host: &str,
    query_type: u16,
    query_label: &str,
) -> DnsProbeOutcome {
    let started = Instant::now();
    let resolver_address = SocketAddr::new(resolver_ip, 53);
    let id = rand::rng().random::<u16>();
    let query = match build_query(id, host, query_type) {
        Ok(value) => value,
        Err(error) => return failed(started, error, "dns_invalid_name"),
    };

    let bind_address = if resolver_ip.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = match UdpSocket::bind(bind_address).await {
        Ok(socket) => socket,
        Err(error) => return failed(started, error.to_string(), "dns_udp_bind"),
    };
    if let Err(error) = socket.send_to(&query, resolver_address).await {
        return failed(started, error.to_string(), "dns_udp_send");
    }
    let mut buffer = vec![0u8; 4096];
    let length = match timeout(DNS_TIMEOUT, socket.recv_from(&mut buffer)).await {
        Ok(Ok((length, source))) if source.ip() == resolver_ip => length,
        Ok(Ok(_)) => {
            return failed(
                started,
                "DNS response came from an unexpected server",
                "dns_unexpected_source",
            )
        }
        Ok(Err(error)) => return failed(started, error.to_string(), "dns_udp_receive"),
        Err(_) => return failed(started, "DNS UDP timeout", "dns_timeout"),
    };
    buffer.truncate(length);

    match parse_response(id, &buffer) {
        Ok(parsed) if parsed.truncated => {
            let mut outcome = probe_tcp(resolver_address, id, query, started).await;
            outcome.detail = format!("{query_label} {}", outcome.detail);
            outcome
        }
        Ok(parsed) => {
            let mut outcome = parsed.into_outcome(started, "UDP");
            outcome.detail = format!("{query_label} {}", outcome.detail);
            outcome
        }
        Err(error) => failed(started, error, "dns_malformed_response"),
    }
}

async fn probe_tcp(
    resolver: SocketAddr,
    id: u16,
    query: Vec<u8>,
    started: Instant,
) -> DnsProbeOutcome {
    let mut stream = match timeout(DNS_TIMEOUT, TcpStream::connect(resolver)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => return failed(started, error.to_string(), "dns_tcp_connect"),
        Err(_) => return failed(started, "DNS TCP connect timeout", "dns_tcp_timeout"),
    };
    let length = (query.len() as u16).to_be_bytes();
    if let Err(error) = stream.write_all(&length).await {
        return failed(started, error.to_string(), "dns_tcp_send");
    }
    if let Err(error) = stream.write_all(&query).await {
        return failed(started, error.to_string(), "dns_tcp_send");
    }
    let mut response_length = [0u8; 2];
    match timeout(DNS_TIMEOUT, stream.read_exact(&mut response_length)).await {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => return failed(started, error.to_string(), "dns_tcp_receive"),
        Err(_) => return failed(started, "DNS TCP response timeout", "dns_tcp_timeout"),
    }
    let response_length = u16::from_be_bytes(response_length) as usize;
    if response_length == 0 {
        return failed(
            started,
            "Invalid DNS TCP response length",
            "dns_malformed_response",
        );
    }
    let mut response = vec![0u8; response_length];
    match timeout(DNS_TIMEOUT, stream.read_exact(&mut response)).await {
        Ok(Ok(_)) => match parse_response(id, &response) {
            Ok(parsed) => parsed.into_outcome(started, "TCP"),
            Err(error) => failed(started, error, "dns_malformed_response"),
        },
        Ok(Err(error)) => failed(started, error.to_string(), "dns_tcp_receive"),
        Err(_) => failed(started, "DNS TCP response timeout", "dns_tcp_timeout"),
    }
}

fn build_query(id: u16, host: &str, query_type: u16) -> Result<Vec<u8>, String> {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty() || host.len() > 253 {
        return Err("Invalid DNS host name".into());
    }
    let mut query = Vec::with_capacity(host.len() + 18);
    query.extend_from_slice(&id.to_be_bytes());
    query.extend_from_slice(&0x0100u16.to_be_bytes());
    query.extend_from_slice(&query_type.to_be_bytes());
    query.extend_from_slice(&0u16.to_be_bytes());
    query.extend_from_slice(&0u16.to_be_bytes());
    query.extend_from_slice(&0u16.to_be_bytes());
    for label in host.split('.') {
        if label.is_empty() || label.len() > 63 || !label.is_ascii() {
            return Err("Invalid DNS host label".into());
        }
        query.push(label.len() as u8);
        query.extend_from_slice(label.as_bytes());
    }
    query.push(0);
    query.extend_from_slice(&1u16.to_be_bytes());
    query.extend_from_slice(&1u16.to_be_bytes());
    Ok(query)
}

struct ParsedDnsResponse {
    truncated: bool,
    rcode: u8,
    address_count: u16,
}

impl ParsedDnsResponse {
    fn into_outcome(self, started: Instant, transport: &str) -> DnsProbeOutcome {
        match self.rcode {
            0 if self.address_count > 0 => passed(
                started,
                format!("DNS {transport}: {} address record(s)", self.address_count),
            ),
            0 => failed(
                started,
                format!("DNS {transport}: no A/AAAA records"),
                "dns_empty",
            ),
            2 => inconclusive(started, "DNS server failure", "dns_servfail"),
            3 => failed(started, "DNS name does not exist", "dns_nxdomain"),
            5 => inconclusive(started, "DNS query refused", "dns_refused"),
            code => failed(started, format!("DNS error code {code}"), "dns_rcode"),
        }
    }
}

fn parse_response(expected_id: u16, response: &[u8]) -> Result<ParsedDnsResponse, String> {
    if response.len() < 12 {
        return Err("DNS response is too short".into());
    }
    let id = u16::from_be_bytes([response[0], response[1]]);
    if id != expected_id {
        return Err("DNS response ID does not match request".into());
    }
    let flags = u16::from_be_bytes([response[2], response[3]]);
    if flags & 0x8000 == 0 {
        return Err("DNS packet is not a response".into());
    }
    let question_count = u16::from_be_bytes([response[4], response[5]]) as usize;
    let answer_count = u16::from_be_bytes([response[6], response[7]]) as usize;
    let mut offset = 12usize;
    for _ in 0..question_count {
        offset = skip_name(response, offset)?;
        offset = offset.checked_add(4).ok_or("DNS question overflow")?;
        if offset > response.len() {
            return Err("Truncated DNS question".into());
        }
    }
    let mut address_count = 0u16;
    for _ in 0..answer_count {
        offset = skip_name(response, offset)?;
        if offset + 10 > response.len() {
            return Err("Truncated DNS answer".into());
        }
        let record_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let data_length = u16::from_be_bytes([response[offset + 8], response[offset + 9]]) as usize;
        offset += 10;
        if offset + data_length > response.len() {
            return Err("Truncated DNS record data".into());
        }
        if (record_type == 1 && data_length == 4) || (record_type == 28 && data_length == 16) {
            address_count = address_count.saturating_add(1);
        }
        offset += data_length;
    }
    Ok(ParsedDnsResponse {
        truncated: flags & 0x0200 != 0,
        rcode: (flags & 0x000f) as u8,
        address_count,
    })
}

fn skip_name(packet: &[u8], mut offset: usize) -> Result<usize, String> {
    loop {
        let length = *packet.get(offset).ok_or("Truncated DNS name")?;
        if length & 0xc0 == 0xc0 {
            if offset + 1 >= packet.len() {
                return Err("Truncated DNS compression pointer".into());
            }
            return Ok(offset + 2);
        }
        offset += 1;
        if length == 0 {
            return Ok(offset);
        }
        if length > 63 || offset + length as usize > packet.len() {
            return Err("Invalid DNS name".into());
        }
        offset += length as usize;
    }
}

fn passed(started: Instant, detail: String) -> DnsProbeOutcome {
    DnsProbeOutcome {
        status: ProbeStatus::Passed,
        latency_ms: started.elapsed().as_millis(),
        detail,
        reason_code: None,
    }
}

fn failed(started: Instant, detail: impl Into<String>, reason: &str) -> DnsProbeOutcome {
    DnsProbeOutcome {
        status: ProbeStatus::Failed,
        latency_ms: started.elapsed().as_millis(),
        detail: detail.into(),
        reason_code: Some(reason.into()),
    }
}

fn inconclusive(started: Instant, detail: impl Into<String>, reason: &str) -> DnsProbeOutcome {
    DnsProbeOutcome {
        status: ProbeStatus::Inconclusive,
        latency_ms: started.elapsed().as_millis(),
        detail: detail.into(),
        reason_code: Some(reason.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_query_and_rejects_bad_names() {
        let query = build_query(7, "discord.com", 1).unwrap();
        assert_eq!(&query[0..2], &7u16.to_be_bytes());
        assert!(build_query(7, "bad..name", 1).is_err());
    }

    #[test]
    fn parses_a_record_response() {
        let id = 0x1234u16;
        let mut response = build_query(id, "example.com", 1).unwrap();
        response[2] = 0x81;
        response[3] = 0x80;
        response[6] = 0;
        response[7] = 1;
        response.extend_from_slice(&[0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01]);
        response.extend_from_slice(&[0, 0, 0, 60, 0, 4, 1, 2, 3, 4]);
        let parsed = parse_response(id, &response).unwrap();
        assert_eq!(parsed.rcode, 0);
        assert_eq!(parsed.address_count, 1);
    }

    #[test]
    #[ignore = "requires live network access"]
    fn live_system_and_direct_dns_probes_return_classified_results() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let system = probe("SYSTEM", "discord.com").await;
            let direct = probe("1.1.1.1", "discord.com").await;
            assert!(!system.detail.is_empty());
            assert!(!direct.detail.is_empty());
            println!("system={:?}: {}", system.status, system.detail);
            println!("direct={:?}: {}", direct.status, direct.detail);
        });
    }
}
