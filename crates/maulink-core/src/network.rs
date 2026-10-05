use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::RwLock, task::spawn_blocking, time::timeout};

use crate::{
    AppError, ErrorCode, NetworkGeo, NetworkHostKind, NetworkInspection, NetworkInspectionSource,
    NetworkIpVersion, NetworkScope,
};

const CACHE_TTL: Duration = Duration::from_secs(10 * 60);
const DNS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
pub struct NetworkInspector {
    cache: Arc<RwLock<HashMap<(String, bool), CachedInspection>>>,
}

#[derive(Clone)]
struct CachedInspection {
    inspection: NetworkInspection,
    expires_at: Instant,
}

impl NetworkInspector {
    pub async fn inspect(&self, host: &str, detailed: bool) -> Result<NetworkInspection, AppError> {
        let normalized_host = crate::host_keys::normalize_host(host)?;
        let key = (normalized_host.clone(), detailed);
        if let Some(cached) = self.cache.read().await.get(&key)
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.inspection.clone());
        }

        let inspection = inspect_uncached(&normalized_host, detailed).await?;
        let now = Instant::now();
        let mut cache = self.cache.write().await;
        cache.retain(|_, cached| cached.expires_at > now);
        cache.insert(
            key,
            CachedInspection {
                inspection: inspection.clone(),
                expires_at: now + CACHE_TTL,
            },
        );
        Ok(inspection)
    }
}

async fn inspect_uncached(host: &str, detailed: bool) -> Result<NetworkInspection, AppError> {
    let input_ip = host.parse::<IpAddr>().ok();
    let host_kind = if input_ip.is_some() {
        NetworkHostKind::Ip
    } else {
        NetworkHostKind::Hostname
    };
    let mut addresses = match input_ip {
        Some(address) => vec![address],
        None if !detailed => Vec::new(),
        None => {
            let dns_name = idna::domain_to_ascii(host).map_err(|_| invalid_host())?;
            if dns_name.is_empty() || dns_name.len() > 253 {
                return Err(invalid_host());
            }
            let resolved = timeout(
                DNS_TIMEOUT,
                spawn_blocking(move || {
                    dns_lookup::lookup_host(&dns_name)
                        .map(|addresses| addresses.collect::<Vec<_>>())
                }),
            )
            .await
            .map_err(|_| dns_failed())?
            .map_err(|_| dns_failed())?
            .map_err(|_| dns_failed())?;
            let mut addresses = resolved;
            addresses.sort_unstable_by_key(ToString::to_string);
            addresses.dedup();
            if addresses.is_empty() {
                return Err(dns_failed());
            }
            addresses
        }
    };

    let primary = addresses.first().copied();
    let reverse_dns = if detailed {
        match primary {
            Some(address) => reverse_dns(address).await,
            None => None,
        }
    } else {
        None
    };
    let (ip_version, scope) = primary.map_or((None, None), |address| {
        let (version, scope) = classify_ip(address);
        (Some(version), Some(scope))
    });

    Ok(NetworkInspection {
        input_host: host.to_owned(),
        host_kind,
        resolved_addresses: addresses
            .drain(..)
            .map(|address| address.to_string())
            .collect(),
        primary_address: primary.map(|address| address.to_string()),
        ip_version,
        scope,
        reverse_dns,
        geo: NetworkGeo {
            country_code: None,
            country_name: None,
            region: None,
            city: None,
        },
        asn: None,
        organization: None,
        source: if detailed {
            NetworkInspectionSource::SystemResolver
        } else {
            NetworkInspectionSource::LocalAnalysis
        },
        database_updated_at_ms: None,
    })
}

async fn reverse_dns(address: IpAddr) -> Option<String> {
    timeout(
        DNS_TIMEOUT,
        spawn_blocking(move || dns_lookup::lookup_addr(&address)),
    )
    .await
    .ok()?
    .ok()?
    .ok()
    .map(|name| name.trim_end_matches('.').to_owned())
}

fn classify_ip(address: IpAddr) -> (NetworkIpVersion, NetworkScope) {
    match address {
        IpAddr::V4(address) => (NetworkIpVersion::Ipv4, classify_ipv4(address)),
        IpAddr::V6(address) => (NetworkIpVersion::Ipv6, classify_ipv6(address)),
    }
}

fn classify_ipv4(address: Ipv4Addr) -> NetworkScope {
    if address.is_unspecified() {
        NetworkScope::Unspecified
    } else if address.is_loopback() {
        NetworkScope::Loopback
    } else if address.is_link_local() {
        NetworkScope::LinkLocal
    } else if address.is_private() {
        NetworkScope::Private
    } else if address.is_multicast() {
        NetworkScope::Multicast
    } else if address.is_broadcast() || is_ipv4_reserved(address) {
        NetworkScope::Reserved
    } else {
        NetworkScope::Public
    }
}

fn is_ipv4_reserved(address: Ipv4Addr) -> bool {
    [
        (192, 0, 0, 0, 24),
        (192, 0, 2, 0, 24),
        (192, 88, 99, 0, 24),
        (198, 18, 0, 0, 15),
        (198, 51, 100, 0, 24),
        (203, 0, 113, 0, 24),
        (240, 0, 0, 0, 4),
        (100, 64, 0, 0, 10),
    ]
    .into_iter()
    .any(|(a, b, c, d, prefix)| ipv4_in_prefix(address, Ipv4Addr::new(a, b, c, d), prefix))
}

fn ipv4_in_prefix(address: Ipv4Addr, network: Ipv4Addr, prefix: u32) -> bool {
    let mask = u32::MAX << (32 - prefix);
    u32::from(address) & mask == u32::from(network) & mask
}

fn classify_ipv6(address: Ipv6Addr) -> NetworkScope {
    if address.is_unspecified() {
        NetworkScope::Unspecified
    } else if address.is_loopback() {
        NetworkScope::Loopback
    } else if address.is_multicast() {
        NetworkScope::Multicast
    } else if address.is_unicast_link_local() {
        NetworkScope::LinkLocal
    } else if is_ipv4_mapped(address) {
        classify_ipv4(mapped_ipv4(address))
    } else if address.segments()[0] & 0xfe00 == 0xfc00 {
        NetworkScope::Private
    } else if is_ipv6_reserved(address) {
        NetworkScope::Reserved
    } else {
        NetworkScope::Public
    }
}

fn is_ipv4_mapped(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    segments[..5] == [0; 5] && segments[5] == u16::MAX
}

fn mapped_ipv4(address: Ipv6Addr) -> Ipv4Addr {
    let segments = address.segments();
    Ipv4Addr::new(
        (segments[6] >> 8) as u8,
        segments[6] as u8,
        (segments[7] >> 8) as u8,
        segments[7] as u8,
    )
}

fn is_ipv6_reserved(address: Ipv6Addr) -> bool {
    [
        (Ipv6Addr::new(0x0100, 0, 0, 0, 0, 0, 0, 0), 64),
        (Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23),
        (Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0), 32),
    ]
    .into_iter()
    .any(|(network, prefix)| ipv6_in_prefix(address, network, prefix))
}

fn ipv6_in_prefix(address: Ipv6Addr, network: Ipv6Addr, prefix: u32) -> bool {
    let mask = u128::MAX << (128 - prefix);
    u128::from(address) & mask == u128::from(network) & mask
}

fn invalid_host() -> AppError {
    AppError::new(ErrorCode::ValidationFailed, "errors.hostInvalid").with_param("field", "host")
}

fn dns_failed() -> AppError {
    AppError::new(ErrorCode::DnsFailed, "errors.dnsFailed").with_stage("networkInspection")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_ipv4_scopes_without_network_access() {
        assert_eq!(
            classify_ip("1.1.1.1".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Public)
        );
        assert_eq!(
            classify_ip("192.168.1.10".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Private)
        );
        assert_eq!(
            classify_ip("127.0.0.1".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Loopback)
        );
        assert_eq!(
            classify_ip("169.254.1.1".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::LinkLocal)
        );
        assert_eq!(
            classify_ip("0.0.0.0".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Unspecified)
        );
        assert_eq!(
            classify_ip("224.0.0.1".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Multicast)
        );
        assert_eq!(
            classify_ip("192.0.2.1".parse().unwrap()),
            (NetworkIpVersion::Ipv4, NetworkScope::Reserved)
        );
    }

    #[test]
    fn classifies_ipv6_scopes_and_mapped_ipv4_addresses() {
        assert_eq!(
            classify_ip("2606:4700:4700::1111".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Public)
        );
        assert_eq!(
            classify_ip("fd00::1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Private)
        );
        assert_eq!(
            classify_ip("::1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Loopback)
        );
        assert_eq!(
            classify_ip("fe80::1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::LinkLocal)
        );
        assert_eq!(
            classify_ip("ff02::1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Multicast)
        );
        assert_eq!(
            classify_ip("2001:db8::1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Reserved)
        );
        assert_eq!(
            classify_ip("::ffff:192.168.1.1".parse().unwrap()),
            (NetworkIpVersion::Ipv6, NetworkScope::Private)
        );
    }

    #[tokio::test]
    async fn classifies_ip_locally_and_does_not_resolve_domains_without_explicit_detail_request() {
        let inspector = NetworkInspector::default();
        let private = inspector
            .inspect("192.168.1.10", false)
            .await
            .expect("private IP");
        assert_eq!(private.host_kind, NetworkHostKind::Ip);
        assert_eq!(private.ip_version, Some(NetworkIpVersion::Ipv4));
        assert_eq!(private.scope, Some(NetworkScope::Private));
        assert_eq!(private.source, NetworkInspectionSource::LocalAnalysis);
        assert_eq!(private.resolved_addresses, ["192.168.1.10"]);
        assert!(private.reverse_dns.is_none());
        assert!(private.geo.country_code.is_none());

        let hostname = inspector
            .inspect("localhost", false)
            .await
            .expect("hostname summary");
        assert_eq!(hostname.host_kind, NetworkHostKind::Hostname);
        assert!(hostname.resolved_addresses.is_empty());
        assert!(hostname.primary_address.is_none());
        assert!(hostname.scope.is_none());
    }

    #[tokio::test]
    async fn resolves_hostnames_only_when_detailed_inspection_is_requested() {
        let inspection = NetworkInspector::default()
            .inspect("localhost", true)
            .await
            .expect("localhost resolves through the system resolver");
        assert_eq!(inspection.host_kind, NetworkHostKind::Hostname);
        assert!(!inspection.resolved_addresses.is_empty());
        assert!(inspection.primary_address.is_some());
        assert_eq!(inspection.source, NetworkInspectionSource::SystemResolver);
    }
}
