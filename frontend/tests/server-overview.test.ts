import { afterEach, describe, expect, it, vi } from "vitest";
import { DOMWrapper, flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import type { ServerProfile } from "../../contracts/v1/ServerProfile";
import type { HostKeyRecord } from "../../contracts/v1/HostKeyRecord";
import type { NetworkInspection } from "../../contracts/v1/NetworkInspection";
import type { ConnectionPreflightResult } from "../../contracts/v1/ConnectionPreflightResult";
import type { ServerRuntimeStats } from "../../contracts/v1/ServerRuntimeStats";
import { locale } from "../src/i18n/locale";
import { createIpcClient } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createConnectionApi } from "../src/ipc/connection";
import { createHostKeysApi } from "../src/ipc/host-keys";
import { createNetworkApi } from "../src/ipc/network";
import { createPreflightApi } from "../src/ipc/preflight";
import { createServerRuntimeStatsApi } from "../src/ipc/server-runtime-stats";
import { createConnectionStore } from "../src/stores/connections";
import ConnectionInfoCard from "../src/components/server-overview/ConnectionInfoCard.vue";
import ConnectionRouteCard from "../src/components/server-overview/ConnectionRouteCard.vue";
import HostIdentityCard from "../src/components/server-overview/HostIdentityCard.vue";
import NetworkInfoCard from "../src/components/server-overview/NetworkInfoCard.vue";
import RecentActivityCard from "../src/components/server-overview/RecentActivityCard.vue";
import ServerOverview from "../src/components/server-overview/ServerOverview.vue";
import { shellServers } from "../src/harness/shell-fixtures";
import { buildSshCommand, connectionRoute } from "../src/components/server-overview/server-details";

const base = shellServers[0]!;
const wrappers: VueWrapper[] = [];
const stores: ReturnType<typeof createConnectionStore>[] = [];
function profile(changes: Partial<ServerProfile> = {}): ServerProfile { return { ...base, ...changes }; }
function connectionStore() {
  const store = createConnectionStore(createConnectionApi(createIpcClient(createMockIpc({}))));
  stores.push(store);
  return store;
}
function hostKeyApi(get: (payload: { host: string; port: number }) => HostKeyRecord | null | Promise<HostKeyRecord | null>) {
  return createHostKeysApi(createIpcClient(createMockIpc({ host_key_get: get })));
}
function networkApi(inspect: (payload: { host: string; detailed: boolean }) => NetworkInspection | Promise<NetworkInspection>) {
  return createNetworkApi(createIpcClient(createMockIpc({ network_inspect: inspect })));
}
function preflightResult(error: ConnectionPreflightResult["error"] = null): ConnectionPreflightResult {
  return {
    resolvedAddresses: ["198.51.100.10"], selectedAddress: error ? null : "198.51.100.10",
    dnsDurationMs: 4, tcpReachable: error ? false : true, tcpConnectDurationMs: 7, error, checkedAtMs: 1_800_000_000_000,
  };
}
function preflightApi(check: (payload: { serverId: string; host: string; port: number; timeoutMs: number }) => ConnectionPreflightResult | Promise<ConnectionPreflightResult> = () => preflightResult()) {
  return createPreflightApi(createIpcClient(createMockIpc({ connection_preflight: check })));
}
function runtimeStats(serverId = base.id): ServerRuntimeStats {
  return { serverId, lastSuccessAtMs: null, lastFailureAtMs: null, lastPreflightAtMs: null, lastPreflightLatencyMs: null, lastFailureCode: null, updatedAtMs: 0 };
}
function runtimeStatsApi(get: (payload: { serverId: string }) => ServerRuntimeStats | Promise<ServerRuntimeStats> = ({ serverId }) => runtimeStats(serverId)) {
  return createServerRuntimeStatsApi(createIpcClient(createMockIpc({ server_runtime_stats_get: get })));
}
const savedHostKey = { normalizedHost: "192.168.1.20", port: 22, algorithm: "ssh-ed25519", fingerprintSha256: "SHA256:fixture-fingerprint", revision: 1, trustedAtMs: 1_790_812_800_000 };
function networkResult(host: string, detailed: boolean): NetworkInspection {
  const isAddress = host === "192.168.1.20";
  return {
    inputHost: host, hostKind: isAddress ? "ip" : "hostname",
    resolvedAddresses: detailed ? [isAddress ? host : "8.8.8.8"] : isAddress ? [host] : [],
    primaryAddress: detailed ? (isAddress ? host : "8.8.8.8") : isAddress ? host : null,
    ipVersion: isAddress || detailed ? "ipv4" : null, scope: isAddress ? "private" : detailed ? "public" : null,
    reverseDns: detailed ? "dns.google" : null,
    geo: { countryCode: null, countryName: null, region: null, city: null }, asn: null, organization: null,
    source: detailed ? "systemResolver" : "localAnalysis", databaseUpdatedAtMs: null, databaseSource: null,
  };
}
function overview(server = profile(), options: {
  getHostKey?: (payload: { host: string; port: number }) => HostKeyRecord | null | Promise<HostKeyRecord | null>;
  inspect?: (payload: { host: string; detailed: boolean }) => NetworkInspection | Promise<NetworkInspection>;
  getStats?: (payload: { serverId: string }) => ServerRuntimeStats | Promise<ServerRuntimeStats>;
} = {}) {
  return mount(ServerOverview, { props: {
    server, store: connectionStore(), hostKeyApi: hostKeyApi(options.getHostKey ?? (() => null)),
    networkApi: networkApi(options.inspect ?? (({ host, detailed }) => networkResult(host, detailed))),
    preflightApi: preflightApi(), runtimeStatsApi: runtimeStatsApi(options.getStats), readOnly: false,
  } });
}
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  stores.splice(0).forEach(store => store.dispose());
  document.body.innerHTML = "";
  locale.value = "zh-CN";
});

describe("server overview", () => {
  it("uses a lightweight breadcrumb, keeps the back action, and avoids a repeated endpoint subtitle", async () => {
    const wrapper = overview(profile({ name: "root@192.168.1.20" }));
    wrappers.push(wrapper);
    const back = wrapper.get(".server-overview-breadcrumb-back");
    expect(wrapper.get(".server-overview-breadcrumb-current").text()).toBe("服务器详情");
    expect(wrapper.get(".server-overview-endpoint").text()).toBe("SSH · 端口 22 · 密码认证");

    await back.trigger("click");
    expect(wrapper.emitted("back")).toHaveLength(1);

    locale.value = "en";
    await flushPromises();
    expect(wrapper.get(".server-overview-breadcrumb-current").text()).toBe("Server details");
    expect(wrapper.get(".server-overview-endpoint").text()).toBe("SSH · Port 22 · Password authentication");
  });

  it("preserves the endpoint subtitle for custom names and handles IPv6 names", () => {
    const ipv6 = overview(profile({ name: "root@[2001:db8::20]", host: "2001:db8::20", authType: "privateKey" }));
    wrappers.push(ipv6);
    expect(ipv6.get(".server-overview-endpoint").text()).toBe("SSH · 端口 22 · 私钥认证");

    const custom = overview(profile({ name: "Production API", host: "2001:db8::20", port: 2222 }));
    wrappers.push(custom);
    expect(custom.get(".server-overview-endpoint").text()).toBe("root@[2001:db8::20]:2222");
  });

  it("shows authentication and saved-credential status from the profile and keeps advanced details collapsed", async () => {
    const wrapper = mount(ConnectionInfoCard, { props: { server: profile({ authType: "privateKey", hasPrivateKey: true, hasSavedCredential: true }) } });
    wrappers.push(wrapper);
    expect(wrapper.text()).toContain("私钥");
    expect(wrapper.text()).toContain("已配置");
    expect(wrapper.text()).toContain("已保存");
    expect(wrapper.get("details").element).toHaveProperty("open", false);
    await wrapper.get("details summary").trigger("click");
    expect(wrapper.get("details").element).toHaveProperty("open", true);
  });

  it("supports the English overview labels", () => {
    locale.value = "en";
    const wrapper = mount(ConnectionInfoCard, { props: { server: profile() } });
    wrappers.push(wrapper);
    expect(wrapper.text()).toContain("Connection information");
    expect(wrapper.text()).toContain("Authentication");
    expect(wrapper.text()).toContain("Saved");
  });

  it("renders direct and jump-host connection paths", () => {
    const direct = mount(ConnectionRouteCard, { props: { server: profile() } });
    wrappers.push(direct);
    expect(direct.findAll(".server-overview-route-step").map(step => step.text())).toEqual(["本机", "→服务器root@192.168.1.20:22"]);

    const jump = mount(ConnectionRouteCard, { props: { server: profile({ jumpHost: "deploy@bastion.example.com", jumpPort: 2222 }) } });
    wrappers.push(jump);
    expect(jump.findAll(".server-overview-route-step").map(step => step.text())).toEqual([
      "本机",
      "→跳板机deploy@bastion.example.com:2222",
      "→服务器root@192.168.1.20:22",
    ]);
  });

  it("keeps proxy before jump host in the displayed route", () => {
    const steps = connectionRoute(profile({
      proxyType: "socks5", proxyHost: "proxy.example.com", proxyPort: 1080,
      jumpHost: "deploy@bastion.example.com", jumpPort: 2222,
    }), { local: "Local", proxy: "Proxy", jumpHost: "Jump", server: "Server", socks5: "SOCKS5", httpConnect: "HTTP CONNECT" });
    expect(steps.map(step => step.label)).toEqual(["Local", "Proxy", "Jump", "Server"]);
    expect(steps[1]?.detail).toBe("SOCKS5 · proxy.example.com:1080");
  });

  it("shows safe recent connection and preflight summary fields", () => {
    const wrapper = mount(RecentActivityCard, { props: {
      stats: { ...runtimeStats(), lastSuccessAtMs: 1_800_000_000_000, lastFailureAtMs: 1_800_000_060_000, lastFailureCode: "AUTH_FAILED", lastPreflightAtMs: 1_800_000_120_000, lastPreflightLatencyMs: 47 },
      state: "ready",
    } });
    wrappers.push(wrapper);
    expect(wrapper.text()).toContain("最近活动");
    expect(wrapper.text()).toContain("上次成功连接");
    expect(wrapper.text()).toContain("上次连接失败");
    expect(wrapper.text()).toContain("AUTH_FAILED");
    expect(wrapper.text()).toContain("47 毫秒");
    expect(wrapper.text()).not.toContain("password");
    expect(wrapper.text()).not.toContain("privateKey");
  });

  it("shows only the saved host key details and clarifies that they are not a live verification", async () => {
    const get = vi.fn(() => Promise.resolve(savedHostKey));
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(get) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(get).toHaveBeenCalledWith({ host: "192.168.1.20", port: 22 });
    expect(wrapper.text()).toContain("已保存信任记录");
    expect(wrapper.text()).toContain("SHA256:fixture-fingerprint");
    expect(wrapper.text()).toContain("连接时仍需校验");
    expect(wrapper.text()).not.toContain("publicKeyBlob");
  });

  it("renders host identity labels in English", async () => {
    locale.value = "en";
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(() => savedHostKey) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("Security & identity");
    expect(wrapper.text()).toContain("Saved trust record");
    expect(wrapper.text()).toContain("It is verified again when connecting");
  });

  it("announces a pending local trust lookup", async () => {
    let resolve!: (record: HostKeyRecord | null) => void;
    const pending = new Promise<HostKeyRecord | null>(done => { resolve = done; });
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(() => pending) } });
    wrappers.push(wrapper);
    expect(wrapper.text()).toContain("正在读取本机信任记录");
    resolve(null);
    await flushPromises();
    expect(wrapper.text()).toContain("尚无已保存的主机身份记录");
  });

  it("shows an empty state when the local device has no saved host key", async () => {
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(() => null) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("尚无已保存的主机身份记录");
    expect(wrapper.text()).toContain("MauLink 将要求确认服务器 Host Key");
  });

  it("keeps host-key read failures local and retries the read", async () => {
    const get = vi.fn().mockRejectedValueOnce(new Error("private storage path")).mockResolvedValueOnce(savedHostKey);
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(get) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("无法读取本地信任记录");
    expect(wrapper.text()).not.toContain("private storage path");
    await wrapper.get("button").trigger("click");
    await flushPromises();
    expect(get).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain("SHA256:fixture-fingerprint");
  });

  it("loads a new record when the selected server endpoint changes", async () => {
    const get = vi.fn(({ host, port }: { host: string; port: number }) => Promise.resolve({ ...savedHostKey, normalizedHost: host, port }));
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(get) } });
    wrappers.push(wrapper);
    await flushPromises();
    await wrapper.setProps({ server: profile({ host: "db.example.com", port: 2222 }) });
    await flushPromises();
    expect(get).toHaveBeenNthCalledWith(2, { host: "db.example.com", port: 2222 });
  });

  it("ignores an older lookup that completes after the endpoint changes", async () => {
    let resolveFirst!: (record: HostKeyRecord | null) => void;
    const first = new Promise<HostKeyRecord | null>(done => { resolveFirst = done; });
    const get = vi.fn(({ host }: { host: string; port: number }) => host === "192.168.1.20"
      ? first
      : Promise.resolve({ ...savedHostKey, normalizedHost: host, fingerprintSha256: "SHA256:new-server" }));
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(get) } });
    wrappers.push(wrapper);
    await wrapper.setProps({ server: profile({ host: "db.example.com", port: 2222 }) });
    await flushPromises();
    resolveFirst({ ...savedHostKey, fingerprintSha256: "SHA256:stale-server" });
    await flushPromises();
    expect(wrapper.text()).toContain("SHA256:new-server");
    expect(wrapper.text()).not.toContain("SHA256:stale-server");
  });

  it("generates only representable OpenSSH commands and quotes shell arguments", () => {
    expect(buildSshCommand(profile())).toBe("ssh -p 22 -- root@192.168.1.20");
    expect(buildSshCommand(profile({ jumpHost: "deploy@bastion.example.com", jumpPort: 2222 }))).toBe("ssh -p 22 -J deploy@bastion.example.com:2222 -- root@192.168.1.20");
    expect(buildSshCommand(profile({ proxyType: "httpConnect", proxyHost: "proxy.example.com", proxyPort: 8080 }))).toBeNull();
    expect(buildSshCommand(profile({ username: "ro'ot" }))).toBe("ssh -p 22 -- 'ro'\\''ot@192.168.1.20'");
  });

  it("disables SSH-command copy when a proxy cannot be represented", async () => {
    const wrapper = mount(ServerOverview, { props: {
      server: profile({ proxyType: "socks5", proxyHost: "proxy.example.com", proxyPort: 1080 }),
      store: connectionStore(), hostKeyApi: hostKeyApi(() => null), networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), preflightApi: preflightApi(), runtimeStatsApi: runtimeStatsApi(), readOnly: false,
    }, attachTo: document.body });
    wrappers.push(wrapper);
    await wrapper.get("[aria-label^='更多服务器操作']").trigger("click");
    await flushPromises();
    const sshItem = new DOMWrapper(document.body).get('[data-action="copy-ssh"]');
    expect(sshItem.attributes("data-disabled")).toBeDefined();
    expect(sshItem.attributes("title")).toContain("当前代理配置无法生成标准 SSH 命令");
    expect(new DOMWrapper(document.body).get('[data-action="delete"]').element.previousElementSibling?.getAttribute("role")).toBe("separator");
  });

  it("keeps the overview available when the saved host-key lookup fails", async () => {
    const wrapper = mount(ServerOverview, { props: {
      server: profile(), store: connectionStore(), hostKeyApi: hostKeyApi(() => { throw new Error("private storage path"); }),
      networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), preflightApi: preflightApi(), runtimeStatsApi: runtimeStatsApi(), readOnly: false,
    } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.find('[aria-label="连接信息"]').exists()).toBe(true);
    expect(wrapper.find('[aria-label="连接路径"]').exists()).toBe(true);
    expect(wrapper.get('[aria-label="安全与身份"]').text()).toContain("无法读取本地信任记录");
    expect(wrapper.find('[aria-label="网络信息"]').exists()).toBe(true);
    expect(wrapper.text()).not.toContain("private storage path");
  });

  it("runs a lightweight preflight only after the user clicks and reports TCP reachability without SSH login", async () => {
    const check = vi.fn(() => preflightResult());
    const getStats = vi.fn(({ serverId }: { serverId: string }) => runtimeStats(serverId));
    const wrapper = mount(ServerOverview, { props: {
      server: profile(), store: connectionStore(), hostKeyApi: hostKeyApi(() => null),
      networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), preflightApi: preflightApi(check), runtimeStatsApi: runtimeStatsApi(getStats), readOnly: false,
    } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(check).not.toHaveBeenCalled();
    expect(wrapper.find('[aria-label="连接检测结果"]').exists()).toBe(false);

    await wrapper.findAll("button").find(button => button.text().includes("连接检测"))!.trigger("click");
    await flushPromises();

    expect(check).toHaveBeenCalledExactlyOnceWith({ serverId: base.id, host: "192.168.1.20", port: 22, timeoutMs: 10000 });
    expect(getStats).toHaveBeenCalledWith({ serverId: base.id });
    expect(wrapper.emitted("testResult")).toEqual([[{ kind: "success", message: "连接检测成功 · 延迟 7 ms" }]]);
    expect(wrapper.find('[aria-label="连接检测结果"]').exists()).toBe(false);
    expect(wrapper.findAll(".server-overview-card")).toHaveLength(5);
  });

  it("shows a refused port from a completed preflight", async () => {
    const wrapper = mount(ServerOverview, { props: {
      server: profile(), store: connectionStore(), hostKeyApi: hostKeyApi(() => null),
      networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)),
      preflightApi: preflightApi(() => preflightResult("connectionRefused")), runtimeStatsApi: runtimeStatsApi(), readOnly: false,
    } });
    wrappers.push(wrapper);
    await wrapper.findAll("button").find(button => button.text().includes("连接检测"))!.trigger("click");
    await flushPromises();
    expect(wrapper.emitted("testResult")).toEqual([[{ kind: "error", message: "连接检测失败 · 延迟 7 ms" }]]);
    expect(wrapper.find('[aria-label="连接检测结果"]').exists()).toBe(false);
  });

  it("automatically analyzes an IP while keeping technical details collapsed until requested", async () => {
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const wrapper = mount(NetworkInfoCard, { props: { server: profile(), api: networkApi(inspect) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(inspect).toHaveBeenCalledExactlyOnceWith({ host: "192.168.1.20", detailed: true });
    expect(wrapper.text()).toContain("192.168.1.20");
    expect(wrapper.text()).toContain("IPv4 · Private");
    expect(inspect).toHaveBeenCalledTimes(1);

    expect(wrapper.get("details").element).toHaveProperty("open", false);
    await wrapper.get("details summary").trigger("click");
    expect(wrapper.get("details").element).toHaveProperty("open", true);
    expect(wrapper.text()).toContain("dns.google");
    expect(wrapper.text()).toContain("私有网络地址不提供公网 GeoIP 信息");
  });

  it("automatically resolves a hostname when the overview opens", async () => {
    const hostname = profile({ host: "server.example.com" });
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const wrapper = mount(NetworkInfoCard, { props: { server: hostname, api: networkApi(inspect) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(inspect).toHaveBeenCalledExactlyOnceWith({ host: "server.example.com", detailed: true });
    expect(wrapper.get("details").element).toHaveProperty("open", false);
    await wrapper.get("details summary").trigger("click");
    expect(wrapper.get("details").element).toHaveProperty("open", true);
    expect(wrapper.text()).toContain("8.8.8.8");
    expect(wrapper.text()).toContain("反向 DNS");
    expect(wrapper.text()).toContain("暂无位置或 ASN 数据");
  });

  it("keeps DNS failures in the network card and lets the user retry", async () => {
    const inspect = vi.fn().mockRejectedValueOnce(new Error("resolver details")).mockImplementation(({ host, detailed }: { host: string; detailed: boolean }) => Promise.resolve(networkResult(host, detailed)));
    const wrapper = mount(NetworkInfoCard, { props: { server: profile({ host: "server.example.com" }), api: networkApi(inspect) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("无法分析网络信息");
    expect(wrapper.text()).not.toContain("resolver details");
    await wrapper.get("button").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("8.8.8.8");
    expect(inspect).toHaveBeenLastCalledWith({ host: "server.example.com", detailed: true });
    expect(inspect).toHaveBeenCalledTimes(2);
  });

  it("starts with one selected category, keeps all five cards mounted, and never clears the detail panel", async () => {
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const getHostKey = vi.fn(() => null);
    const wrapper = overview(profile(), { inspect, getHostKey });
    wrappers.push(wrapper);
    await flushPromises();

    const buttons = wrapper.findAll(".server-overview-section-button");
    const panels = wrapper.findAll(".server-overview-section-panel");
    expect(buttons).toHaveLength(5);
    expect(buttons.map(button => button.attributes("aria-current"))).toEqual(["true", undefined, undefined, undefined, undefined]);
    expect(buttons.map(button => button.attributes("aria-controls"))).toEqual(panels.map(panel => panel.attributes("id")));
    expect(panels.map(panel => (panel.element as HTMLElement).style.display)).toEqual(["", "none", "none", "none", "none"]);
    expect(panels.map(panel => panel.isVisible())).toEqual([true, false, false, false, false]);
    expect(wrapper.findAll(".server-overview-card")).toHaveLength(5);

    await buttons[1]!.trigger("click");
    expect(buttons.map(button => button.attributes("aria-current"))).toEqual([undefined, "true", undefined, undefined, undefined]);
    expect(panels.map(panel => (panel.element as HTMLElement).style.display)).toEqual(["none", "", "none", "none", "none"]);
    await buttons[1]!.trigger("click");
    expect(buttons.map(button => button.attributes("aria-current"))).toEqual([undefined, "true", undefined, undefined, undefined]);
    expect(panels.map(panel => (panel.element as HTMLElement).style.display)).toEqual(["none", "", "none", "none", "none"]);
    await buttons[2]!.trigger("click");
    await buttons[3]!.trigger("click");
    await buttons[4]!.trigger("click");
    expect(inspect).toHaveBeenCalledExactlyOnceWith({ host: base.host, detailed: true });
    expect(getHostKey).toHaveBeenCalledExactlyOnceWith({ host: base.host, port: base.port });
  });

  it("keeps hidden error controls out of view and exposes them when their category is selected", async () => {
    const wrapper = overview(profile({ host: "public.example.test" }), {
      inspect: () => Promise.reject(new Error("resolver details")),
    });
    wrappers.push(wrapper);
    document.body.appendChild(wrapper.element);
    await flushPromises();

    const networkButton = wrapper.get('.server-overview-section-button[data-section="network"]');
    const retry = wrapper.get('[id$="-network-panel"] button');
    expect(networkButton.element.tagName).toBe("BUTTON");
    expect(networkButton.attributes("role")).toBeUndefined();
    expect(retry.isVisible()).toBe(false);

    await networkButton.trigger("click");
    expect(retry.isVisible()).toBe(true);
    expect(wrapper.get(".server-overview-detail-panel-heading h3").text()).toBe("网络信息");
  });

  it("keeps the selected category during endpoint edits and refreshes network and identity data", async () => {
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const getHostKey = vi.fn(({ host, port }: { host: string; port: number }) => ({ ...savedHostKey, normalizedHost: host, port }));
    const wrapper = overview(profile({ id: "same-server" }), { inspect, getHostKey });
    wrappers.push(wrapper);
    await flushPromises();
    await wrapper.get('.server-overview-section-button[data-section="network"]').trigger("click");

    await wrapper.setProps({ server: profile({ id: "same-server", host: "db.example.test", port: 2222 }) });
    await flushPromises();

    expect(wrapper.get('.server-overview-section-button[data-section="network"]').attributes("aria-current")).toBe("true");
    expect(inspect).toHaveBeenNthCalledWith(2, { host: "db.example.test", detailed: true });
    expect(getHostKey).toHaveBeenNthCalledWith(2, { host: "db.example.test", port: 2222 });
    expect(wrapper.get('[aria-label="安全与身份"]').text()).toContain("SHA256:fixture-fingerprint");
  });

  it("resets to connection and rejects stale network and trust summaries after switching servers", async () => {
    let resolveFirstNetwork!: (inspection: NetworkInspection) => void;
    let resolveFirstTrust!: (record: HostKeyRecord | null) => void;
    const firstNetwork = new Promise<NetworkInspection>(resolve => { resolveFirstNetwork = resolve; });
    const firstTrust = new Promise<HostKeyRecord | null>(resolve => { resolveFirstTrust = resolve; });
    const secondNetwork: NetworkInspection = {
      ...networkResult("b.example.test", true),
      primaryAddress: "203.0.113.10",
      geo: { countryCode: "ZZ", countryName: "Region B", region: null, city: null },
    };
    const secondTrust = { ...savedHostKey, normalizedHost: "b.example.test", fingerprintSha256: "SHA256:server-b" };
    const inspect = vi.fn(({ host }: { host: string; detailed: boolean }) => host === "a.example.test" ? firstNetwork : Promise.resolve(secondNetwork));
    const getHostKey = vi.fn(({ host }: { host: string; port: number }) => host === "a.example.test" ? firstTrust : Promise.resolve(secondTrust));
    const wrapper = overview(profile({ id: "server-a", host: "a.example.test" }), { inspect, getHostKey });
    wrappers.push(wrapper);
    const networkButton = wrapper.get('.server-overview-section-button[data-section="network"]');
    await networkButton.trigger("click");

    await wrapper.setProps({ server: profile({ id: "server-b", host: "b.example.test" }) });
    await flushPromises();
    expect(wrapper.findAll(".server-overview-section-button").map(button => button.attributes("aria-current"))).toEqual(["true", undefined, undefined, undefined, undefined]);
    expect(wrapper.text()).toContain("Region B");
    expect(wrapper.text()).toContain("SHA256:server-b");

    resolveFirstNetwork({ ...networkResult("a.example.test", true), geo: { countryCode: "AA", countryName: "Stale A", region: null, city: null } });
    resolveFirstTrust({ ...savedHostKey, fingerprintSha256: "SHA256:stale-a" });
    await flushPromises();
    expect(wrapper.text()).toContain("Region B");
    expect(wrapper.text()).not.toContain("Stale A");
    expect(wrapper.text()).toContain("SHA256:server-b");
    expect(wrapper.text()).not.toContain("SHA256:stale-a");
  });

  it("shows real network classifications and trust read states in the overview summary", async () => {
    const wrapper = overview(profile(), {
      inspect: () => networkResult("192.168.1.20", true),
      getHostKey: () => null,
    });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.findAll(".server-overview-summary-item")[1]!.text()).toContain("IPv4 · Private");
    expect(wrapper.findAll(".server-overview-summary-item")[1]!.text()).not.toContain("Hong Kong");
    expect(wrapper.findAll(".server-overview-summary-item")[2]!.text()).toContain("未保存");
    expect(wrapper.get('[aria-label="安全与身份"]').text()).toContain("尚无已保存的主机身份记录");
  });

  it("summarizes a failed network and trust lookup without inventing GeoIP data", async () => {
    const wrapper = overview(profile(), {
      inspect: () => Promise.reject(new Error("network failure")),
      getHostKey: () => Promise.reject(new Error("local storage failure")),
    });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.findAll(".server-overview-summary-item")[1]!.text()).toContain("分析失败");
    expect(wrapper.findAll(".server-overview-summary-item")[1]!.text()).not.toContain("地区");
    expect(wrapper.findAll(".server-overview-summary-item")[2]!.text()).toContain("读取失败");
    expect(wrapper.text()).not.toContain("local storage failure");
  });

  it("keeps the existing offline GeoIP settings and address-copy actions available", async () => {
    const server = profile({ host: "public.example.test" });
    const wrapper = mount(ServerOverview, { props: {
      server, store: connectionStore(), hostKeyApi: hostKeyApi(() => null),
      networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), preflightApi: preflightApi(), runtimeStatsApi: runtimeStatsApi(), readOnly: false,
    }, attachTo: document.body });
    wrappers.push(wrapper);
    await flushPromises();
    await wrapper.get('.server-overview-section-button[data-section="network"]').trigger("click");
    await wrapper.get(".server-overview-network-configure button").trigger("click");
    expect(wrapper.emitted("openSettings")).toHaveLength(1);

    await wrapper.get('[aria-label^="更多服务器操作"]').trigger("click");
    const actions = new DOMWrapper(document.body);
    await actions.get('[data-action="copy-address"]').trigger("click");
    await flushPromises();
    expect(wrapper.emitted("copy")).toEqual([["address", "root@public.example.test:22"]]);
  });

  it("keeps long host and fingerprint values available without truncating their text", async () => {
    const host = `${"long-host-".repeat(9)}example.test`;
    const fingerprint = `SHA256:${"long-fingerprint-".repeat(8)}`;
    const wrapper = overview(profile({ host }), {
      getHostKey: () => ({ ...savedHostKey, normalizedHost: host, fingerprintSha256: fingerprint }),
    });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.get(".server-overview-fingerprint").text()).toBe(fingerprint);
    expect(wrapper.text()).toContain(host);
    expect(wrapper.get(".server-overview-fingerprint").text()).not.toContain("…");
  });

  it("uses the existing route helper order and summarizes the latest real activity", async () => {
    const server = profile({ proxyType: "socks5", proxyHost: "proxy.example.test", proxyPort: 1080, jumpHost: "deploy@bastion.example.test", jumpPort: 2222 });
    const wrapper = overview(server, { getStats: () => ({
      ...runtimeStats(server.id), lastSuccessAtMs: 1_800_000_000_000, lastPreflightAtMs: 1_800_000_120_000, lastPreflightLatencyMs: 47,
    }) });
    wrappers.push(wrapper);
    await flushPromises();
    await wrapper.get('.server-overview-section-button[data-section="route"]').trigger("click");
    const routeSteps = wrapper.get('[id$="-route-panel"]').findAll(".server-overview-route-node strong").map(node => node.text());
    expect(routeSteps).toEqual(["本机", "代理", "跳板机", "服务器"]);
    await wrapper.get('.server-overview-section-button[data-section="activity"]').trigger("click");
    expect(wrapper.get('[id$="-activity-panel"]').text()).toContain("47 毫秒");
    expect(wrapper.text()).not.toContain("password");
  });

  it("distinguishes empty recent activity from a failed activity read", async () => {
    const empty = overview(profile(), { getStats: ({ serverId }) => runtimeStats(serverId) });
    wrappers.push(empty);
    await flushPromises();
    await empty.get('.server-overview-section-button[data-section="activity"]').trigger("click");
    expect(empty.get('[id$="-activity-panel"]').text()).toContain("暂无连接活动记录");

    const failed = overview(profile({ id: "server-stats-error" }), { getStats: () => Promise.reject(new Error("stats unavailable")) });
    wrappers.push(failed);
    await flushPromises();
    const activityButton = failed.get('.server-overview-section-button[data-section="activity"]');
    await activityButton.trigger("click");
    expect(failed.get('[id$="-activity-panel"]').text()).toContain("无法读取最近活动");
    expect(failed.get('[id$="-activity-panel"]').text()).not.toContain("暂无连接活动记录");
  });

  it("updates all summary labels when the locale changes", async () => {
    const wrapper = overview();
    wrappers.push(wrapper);
    await flushPromises();
    locale.value = "en";
    await flushPromises();
    expect(wrapper.get(".server-overview-summary").attributes("aria-label")).toBe("Server summary");
    expect(wrapper.get('.server-overview-section-button[data-section="connection"]').text()).toContain("Connection configuration");
    expect(wrapper.get(".server-overview-detail-panel-heading h3").text()).toBe("Connection configuration");
    expect(wrapper.get(".server-overview-detail-panel-heading p").text()).toBe("Configuration and diagnostics");
    expect(wrapper.findAll(".server-overview-summary-item")[2]!.text()).toContain("Not saved");
  });
});
