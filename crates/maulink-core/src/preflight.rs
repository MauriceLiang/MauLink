use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tokio::{
    net::{TcpStream, lookup_host},
    time,
};

use crate::{
    AppError, ConnectionPreflightError, ConnectionPreflightPayload, ConnectionPreflightResult,
    ErrorCode,
};

const MIN_TIMEOUT_MS: u32 = 1_000;
const MAX_TIMEOUT_MS: u32 = 120_000;

pub async fn check(
    payload: ConnectionPreflightPayload,
) -> Result<ConnectionPreflightResult, AppError> {
    let host = crate::host_keys::normalize_host(&payload.host)?;
    if payload.port == 0 {
        return Err(validation("port", "errors.portOutOfRange"));
    }
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&payload.timeout_ms) {
        return Err(validation("timeoutMs", "errors.connectTimeoutOutOfRange"));
    }

    let deadline = time::Instant::now() + Duration::from_millis(u64::from(payload.timeout_ms));
    let (addresses, dns_duration_ms) =
        if let Ok(ip) = host.parse::<IpAddr>() {
            (vec![SocketAddr::new(ip, payload.port)], 0)
        } else {
            let ascii_host = match idna::domain_to_ascii(&host) {
                Ok(host) if !host.is_empty() && host.len() <= 253 => host,
                _ => {
                    return Ok(failed_result(
                        Vec::new(),
                        0,
                        None,
                        None,
                        ConnectionPreflightError::DnsFailed,
                    ));
                }
            };
            let dns_started = Instant::now();
            let remaining = deadline.saturating_duration_since(time::Instant::now());
            if remaining.is_zero() {
                return Ok(failed_result(
                    Vec::new(),
                    elapsed_ms(dns_started),
                    None,
                    None,
                    ConnectionPreflightError::Timeout,
                ));
            }
            let addresses =
                match time::timeout(remaining, lookup_host((ascii_host.as_str(), payload.port)))
                    .await
                {
                    Err(_) => {
                        return Ok(failed_result(
                            Vec::new(),
                            elapsed_ms(dns_started),
                            None,
                            None,
                            ConnectionPreflightError::Timeout,
                        ));
                    }
                    Ok(Err(_)) => {
                        return Ok(failed_result(
                            Vec::new(),
                            elapsed_ms(dns_started),
                            None,
                            None,
                            ConnectionPreflightError::DnsFailed,
                        ));
                    }
                    Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
                };
            (addresses, elapsed_ms(dns_started))
        };
    let mut addresses = addresses;
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() {
        return Ok(failed_result(
            addresses,
            dns_duration_ms,
            None,
            None,
            ConnectionPreflightError::DnsFailed,
        ));
    }

    let tcp_started = Instant::now();
    let mut last_error = None;
    let mut attempted = false;
    for address in addresses.iter().copied() {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        if remaining.is_zero() {
            return Ok(failed_result(
                addresses,
                dns_duration_ms,
                attempted.then_some(false),
                attempted.then(|| elapsed_ms(tcp_started)),
                ConnectionPreflightError::Timeout,
            ));
        }
        attempted = true;
        match time::timeout(remaining, TcpStream::connect(address)).await {
            Err(_) => {
                return Ok(failed_result(
                    addresses,
                    dns_duration_ms,
                    Some(false),
                    Some(elapsed_ms(tcp_started)),
                    ConnectionPreflightError::Timeout,
                ));
            }
            Ok(Ok(stream)) => {
                drop(stream);
                return Ok(ConnectionPreflightResult {
                    resolved_addresses: address_strings(&addresses),
                    selected_address: Some(address.ip().to_string()),
                    dns_duration_ms,
                    tcp_reachable: Some(true),
                    tcp_connect_duration_ms: Some(elapsed_ms(tcp_started)),
                    error: None,
                    checked_at_ms: timestamp_ms(),
                });
            }
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                last_error = Some(ConnectionPreflightError::ConnectionRefused);
            }
            Ok(Err(_)) => last_error = Some(ConnectionPreflightError::ConnectionFailed),
        }
    }

    Ok(failed_result(
        addresses,
        dns_duration_ms,
        Some(false),
        Some(elapsed_ms(tcp_started)),
        last_error.unwrap_or(ConnectionPreflightError::ConnectionFailed),
    ))
}

fn failed_result(
    addresses: Vec<SocketAddr>,
    dns_duration_ms: u64,
    tcp_reachable: Option<bool>,
    tcp_connect_duration_ms: Option<u64>,
    error: ConnectionPreflightError,
) -> ConnectionPreflightResult {
    ConnectionPreflightResult {
        resolved_addresses: address_strings(&addresses),
        selected_address: None,
        dns_duration_ms,
        tcp_reachable,
        tcp_connect_duration_ms,
        error: Some(error),
        checked_at_ms: timestamp_ms(),
    }
}

fn address_strings(addresses: &[SocketAddr]) -> Vec<String> {
    addresses
        .iter()
        .map(|address| address.ip().to_string())
        .collect()
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

#[cfg(test)]
mod tests {
    use tokio::{io::AsyncReadExt, net::TcpListener};

    use super::*;

    fn payload(host: &str, port: u16) -> ConnectionPreflightPayload {
        ConnectionPreflightPayload {
            host: host.to_owned(),
            port,
            timeout_ms: MIN_TIMEOUT_MS,
        }
    }

    #[tokio::test]
    async fn connects_to_the_tcp_port_without_performing_an_ssh_handshake() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let accept = tokio::spawn(async move { listener.accept().await.unwrap() });

        let result = check(payload("127.0.0.1", port)).await.unwrap();
        let (mut socket, _) = accept.await.unwrap();

        assert_eq!(result.resolved_addresses, ["127.0.0.1"]);
        assert_eq!(result.selected_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(result.dns_duration_ms, 0);
        assert_eq!(result.tcp_reachable, Some(true));
        assert!(result.tcp_connect_duration_ms.is_some());
        assert_eq!(result.error, None);
        assert!(result.checked_at_ms > 0);
        let mut received = [0; 32];
        assert_eq!(socket.read(&mut received).await.unwrap(), 0);
        drop(socket);
    }

    #[tokio::test]
    async fn resolves_a_hostname_then_checks_the_tcp_port() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let accept = tokio::spawn(async move { listener.accept().await.unwrap() });

        let result = check(payload("localhost", port)).await.unwrap();
        let (socket, _) = accept.await.unwrap();

        assert!(!result.resolved_addresses.is_empty());
        assert!(result.dns_duration_ms < u64::from(MIN_TIMEOUT_MS));
        assert_eq!(result.tcp_reachable, Some(true));
        assert_eq!(result.selected_address.as_deref(), Some("127.0.0.1"));
        drop(socket);
    }

    #[tokio::test]
    async fn reports_a_refused_port_as_a_completed_preflight_result() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let result = check(payload("127.0.0.1", port)).await.unwrap();

        assert_eq!(result.tcp_reachable, Some(false));
        assert_eq!(
            result.error,
            Some(ConnectionPreflightError::ConnectionRefused)
        );
        assert!(result.tcp_connect_duration_ms.is_some());
        assert!(result.selected_address.is_none());
    }

    #[tokio::test]
    async fn rejects_invalid_port_and_timeout_values() {
        assert_eq!(
            check(payload("127.0.0.1", 0)).await.unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        let mut invalid_timeout = payload("127.0.0.1", 22);
        invalid_timeout.timeout_ms = MAX_TIMEOUT_MS + 1;
        assert_eq!(
            check(invalid_timeout).await.unwrap_err().code,
            ErrorCode::ValidationFailed
        );
    }
}
