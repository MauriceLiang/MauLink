import type { NetworkInspection } from "../../../contracts/v1/NetworkInspection";

export function networkFixture(host: string, detailed: boolean): NetworkInspection {
  const isIpv6 = host.includes(":");
  const isIpv4 = /^\d{1,3}(?:\.\d{1,3}){3}$/.test(host);
  const hostKind = isIpv4 || isIpv6 ? "ip" : "hostname";
  const address = isIpv4 || isIpv6 ? host : "198.51.100.10";
  const hasAddress = hostKind === "ip" || detailed;
  return {
    inputHost: host,
    hostKind,
    resolvedAddresses: hasAddress ? [address] : [],
    primaryAddress: hasAddress ? address : null,
    ipVersion: hasAddress ? (isIpv6 ? "ipv6" : "ipv4") : null,
    scope: hasAddress ? (address.startsWith("192.168.") ? "private" : "reserved") : null,
    reverseDns: detailed ? "fixture.example.test" : null,
    geo: { countryCode: null, countryName: null, region: null, city: null },
    asn: null,
    organization: null,
    source: detailed ? "systemResolver" : "localAnalysis",
    databaseUpdatedAtMs: null,
  };
}
