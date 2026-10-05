import { afterEach, describe, expect, it } from "vitest";
import { DOMWrapper, flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import type { ServerProfile } from "../../contracts/v1/ServerProfile";
import { locale } from "../src/i18n/locale";
import { createIpcClient } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createConnectionApi } from "../src/ipc/connection";
import { createConnectionStore } from "../src/stores/connections";
import ConnectionInfoCard from "../src/components/server-overview/ConnectionInfoCard.vue";
import ConnectionRouteCard from "../src/components/server-overview/ConnectionRouteCard.vue";
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

  it("generates only representable OpenSSH commands and quotes shell arguments", () => {
    expect(buildSshCommand(profile())).toBe("ssh -p 22 -- root@192.168.1.20");
    expect(buildSshCommand(profile({ jumpHost: "deploy@bastion.example.com", jumpPort: 2222 }))).toBe("ssh -p 22 -J deploy@bastion.example.com:2222 -- root@192.168.1.20");
    expect(buildSshCommand(profile({ proxyType: "httpConnect", proxyHost: "proxy.example.com", proxyPort: 8080 }))).toBeNull();
    expect(buildSshCommand(profile({ username: "ro'ot" }))).toBe("ssh -p 22 -- 'ro'\\''ot@192.168.1.20'");
  });

  it("disables SSH-command copy when a proxy cannot be represented", async () => {
    const wrapper = mount(ServerOverview, { props: {
      server: profile({ proxyType: "socks5", proxyHost: "proxy.example.com", proxyPort: 1080 }),
      store: connectionStore(), readOnly: false,
    }, attachTo: document.body });
    wrappers.push(wrapper);
    await wrapper.get("[aria-label^='更多服务器操作']").trigger("click");
    await flushPromises();
    const sshItem = new DOMWrapper(document.body).get('[data-action="copy-ssh"]');
    expect(sshItem.attributes("data-disabled")).toBeDefined();
    expect(sshItem.attributes("title")).toContain("当前代理配置无法生成标准 SSH 命令");
    expect(new DOMWrapper(document.body).get('[data-action="delete"]').element.previousElementSibling?.getAttribute("role")).toBe("separator");
  });
});
