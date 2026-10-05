import { afterEach, describe, expect, it, vi } from "vitest";
import { DOMWrapper, flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import type { ServerProfile } from "../../contracts/v1/ServerProfile";
import type { HostKeyRecord } from "../../contracts/v1/HostKeyRecord";
import type { NetworkInspection } from "../../contracts/v1/NetworkInspection";
import { locale } from "../src/i18n/locale";
import { createIpcClient } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createConnectionApi } from "../src/ipc/connection";
import { createHostKeysApi } from "../src/ipc/host-keys";
import { createNetworkApi } from "../src/ipc/network";
import { createConnectionStore } from "../src/stores/connections";
import ConnectionInfoCard from "../src/components/server-overview/ConnectionInfoCard.vue";
import ConnectionRouteCard from "../src/components/server-overview/ConnectionRouteCard.vue";
import HostIdentityCard from "../src/components/server-overview/HostIdentityCard.vue";
import NetworkInfoCard from "../src/components/server-overview/NetworkInfoCard.vue";
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
    source: detailed ? "systemResolver" : "localAnalysis", databaseUpdatedAtMs: null,
  };
}
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  stores.splice(0).forEach(store => store.dispose());
  document.body.innerHTML = "";
  locale.value = "zh-CN";
});

describe("server overview", () => {
  it("shows authentication and saved-credential status from the profile", () => {
    const wrapper = mount(ConnectionInfoCard, { props: { server: profile({ authType: "privateKey", hasPrivateKey: true, hasSavedCredential: true }) } });
    wrappers.push(wrapper);
    expect(wrapper.text()).toContain("私钥");
    expect(wrapper.text()).toContain("已配置");
    expect(wrapper.text()).toContain("已保存");
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

  it("shows only the saved host key details and clarifies that they are not a live verification", async () => {
    const get = vi.fn(() => Promise.resolve(savedHostKey));
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(get) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(get).toHaveBeenCalledWith({ host: "192.168.1.20", port: 22 });
    expect(wrapper.text()).toContain("已保存信任记录");
    expect(wrapper.text()).toContain("SHA256:fixture-fingerprint");
    expect(wrapper.text()).toContain("不代表当前远程服务器已经验证");
    expect(wrapper.text()).not.toContain("publicKeyBlob");
  });

  it("renders host identity labels in English", async () => {
    locale.value = "en";
    const wrapper = mount(HostIdentityCard, { props: { server: profile(), api: hostKeyApi(() => savedHostKey) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.text()).toContain("Security & identity");
    expect(wrapper.text()).toContain("Saved trust record");
    expect(wrapper.text()).toContain("does not mean the current remote server has been verified");
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
      store: connectionStore(), hostKeyApi: hostKeyApi(() => null), networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), readOnly: false,
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
      networkApi: networkApi(({ host, detailed }) => networkResult(host, detailed)), readOnly: false,
    } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(wrapper.find('[aria-label="连接信息"]').exists()).toBe(true);
    expect(wrapper.find('[aria-label="连接路径"]').exists()).toBe(true);
    expect(wrapper.get('[aria-label="安全与身份"]').text()).toContain("无法读取本地信任记录");
    expect(wrapper.find('[aria-label="网络信息"]').exists()).toBe(true);
    expect(wrapper.text()).not.toContain("private storage path");
  });

  it("classifies an IP locally and performs detailed lookup only after explicit action", async () => {
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const wrapper = mount(NetworkInfoCard, { props: { server: profile(), api: networkApi(inspect) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(inspect).toHaveBeenCalledExactlyOnceWith({ host: "192.168.1.20", detailed: false });
    expect(wrapper.text()).toContain("192.168.1.20");
    expect(wrapper.text()).toContain("IPv4 · Private");
    expect(inspect).toHaveBeenCalledTimes(1);

    await wrapper.get("button").trigger("click");
    await flushPromises();
    expect(inspect).toHaveBeenLastCalledWith({ host: "192.168.1.20", detailed: true });
    expect(wrapper.text()).toContain("dns.google");
    expect(wrapper.text()).toContain("私有网络地址不提供公网 GeoIP 信息");
  });

  it("shows a hostname without resolving it until the user asks for analysis", async () => {
    const hostname = profile({ host: "server.example.com" });
    const inspect = vi.fn(({ host, detailed }: { host: string; detailed: boolean }) => networkResult(host, detailed));
    const wrapper = mount(NetworkInfoCard, { props: { server: hostname, api: networkApi(inspect) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(inspect).toHaveBeenCalledExactlyOnceWith({ host: "server.example.com", detailed: false });
    expect(wrapper.text()).toContain("Hostname");
    expect(wrapper.text()).not.toContain("8.8.8.8");
    await wrapper.get("button").trigger("click");
    await flushPromises();
    expect(inspect).toHaveBeenLastCalledWith({ host: "server.example.com", detailed: true });
    expect(wrapper.text()).toContain("8.8.8.8");
    expect(wrapper.text()).toContain("反向 DNS");
    expect(wrapper.text()).toContain("本机未配置离线 GeoIP 数据库");
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
    expect(wrapper.text()).toContain("Hostname");
    expect(inspect).toHaveBeenCalledTimes(2);
  });
});
