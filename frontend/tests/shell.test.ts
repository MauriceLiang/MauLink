import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import AppShell from "../src/app/AppShell.vue";
import { createIpcClient } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createShellMock, shellAppInfo, shellGroups, shellServers } from "../src/harness/shell-fixtures";
import desktopConfig from "../../src-tauri/tauri.conf.json";
import frontendV2Config from "../../src-tauri/tauri.frontend-v2.conf.json";
import ipcHarnessConfig from "../../src-tauri/tauri.harness.conf.json";

const wrappers: VueWrapper[] = [];
function mountShell(transport = createShellMock()) {
  const wrapper = mount(AppShell, { attachTo: document.body, props: { client: createIpcClient(transport) } });
  wrappers.push(wrapper);
  return wrapper;
}
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = "";
});

describe("application shell", () => {
  it("marks the topbar for native dragging and grants only drag access to the main window", () => {
    const wrapper = mountShell();
    expect(wrapper.get("header").attributes()).toHaveProperty("data-tauri-drag-region");
    expect(wrapper.get(".shell-brand").attributes()).not.toHaveProperty("data-tauri-drag-region");
    expect(desktopConfig.app.security).toHaveProperty("capabilities", [
      "main",
      {
        identifier: "main-window-drag",
        windows: ["main"],
        permissions: ["core:window:allow-start-dragging"],
      },
    ]);
    expect(frontendV2Config.app.security).not.toHaveProperty("capabilities");
  });

  it("loads the Vite build with modular IPC and keeps the static IPC harness independent", () => {
    expect(desktopConfig.build).toMatchObject({ frontendDist: "../frontend/dist", devUrl: "http://127.0.0.1:1420", beforeDevCommand: "npm run dev", beforeBuildCommand: "npm run build" });
    expect(desktopConfig.app.withGlobalTauri).toBe(false);
    expect(frontendV2Config.build.frontendDist).toBe(desktopConfig.build.frontendDist);
    expect(ipcHarnessConfig.build).toMatchObject({ frontendDist: "../tools/ipc-harness", devUrl: null, beforeDevCommand: "", beforeBuildCommand: "" });
    expect(ipcHarnessConfig.app.withGlobalTauri).toBe(true);
  });

  it("does not show an empty home or a ready backend before IPC finishes", async () => {
    let resolveInfo!: (value: typeof shellAppInfo) => void;
    const info = new Promise<typeof shellAppInfo>(resolve => { resolveInfo = resolve; });
    const wrapper = mountShell(createMockIpc({ app_get_info: () => info, group_list: () => [], server_list: () => ({ items: [], nextCursor: null }), server_appearance_list: () => [] }));
    expect(wrapper.get("main").text()).toContain("正在加载本地数据");
    expect(wrapper.get('[role="status"]').text()).toContain("正在连接本地服务");
    expect(wrapper.find(".shell-welcome").exists()).toBe(false);
    resolveInfo(shellAppInfo);
    await flushPromises();
    expect(wrapper.get("h1").text()).toBe("还没有服务器");
    expect(wrapper.get('[role="status"]').text()).toContain("本地服务已就绪");
    expect(wrapper.get('[role="status"]').text()).toContain("v0.1.0");
    expect(wrapper.get('[aria-label="设置"]').element).toHaveProperty("disabled", false);
    expect(wrapper.get("main button").element).toHaveProperty("disabled", false);
  });

  it("loads every navigation page without connecting SSH and marks home inactive for Core monitoring", async () => {
    const activity = vi.fn();
    const mock = createMockIpc({
      workspace_set_activity: activity,
      app_get_info: () => shellAppInfo, group_list: () => shellGroups,
      server_appearance_list: () => [],
      host_key_get: () => null,
      network_inspect: ({ host, detailed }) => ({
        inputHost: host, hostKind: "ip", resolvedAddresses: [host], primaryAddress: host, ipVersion: "ipv4", scope: "private",
        reverseDns: null, geo: { countryCode: null, countryName: null, region: null, city: null }, asn: null, organization: null,
        source: detailed ? "systemResolver" : "localAnalysis", databaseUpdatedAtMs: null,
      }),
      connection_preflight: ({ host }) => ({
        resolvedAddresses: [host], selectedAddress: host, dnsDurationMs: 0, tcpReachable: true,
        tcpConnectDurationMs: 1, error: null, checkedAtMs: 1_800_000_000_000,
      }),
      server_list: payload => payload.cursor === null
        ? { items: [shellServers[0]!], nextCursor: "page-2" }
        : { items: [shellServers[1]!, shellServers[2]!], nextCursor: null },
    });
    const commands: string[] = [];
    const wrapper = mountShell({ invoke: async <T,>(command: string, args?: Record<string, unknown>) => {
      commands.push(command);
      return mock.invoke<T>(command, args);
    } });
    await flushPromises();
    expect(wrapper.findAll(".shell-server-item")).toHaveLength(3);
    expect(commands.filter(command => command !== "workspace_set_activity" && command !== "settings_get")).toEqual(["app_get_info", "group_list", "server_list", "server_list", "server_appearance_list"]);
    expect(activity).toHaveBeenCalledTimes(1);
    expect(activity).toHaveBeenCalledWith({ activeConnectionId: null, monitorVisible: false });
    await wrapper.get('[aria-label="Web-01 · 192.168.1.20"]').trigger("click");
    expect(wrapper.get(".server-overview-endpoint").text()).toBe("root@192.168.1.20:22");
    expect(wrapper.get(".server-overview-title-line .base-status-badge").text()).toBe("尚未连接");
    expect(wrapper.get('[aria-label="Web-01 · 192.168.1.20"]').attributes("aria-current")).toBe("page");
    expect(commands.filter(command => command === "settings_get")).toHaveLength(1);
    expect(commands).toHaveLength(9);
    await wrapper.get('[aria-label="服务器"]').trigger("click");
    expect(wrapper.get("h1").text()).toBe("服务器");
    expect(wrapper.find('.shell-server-item[aria-current="page"]').exists()).toBe(false);
  });

  it("keeps global/sidebar search in sync and preserves unknown groups as ungrouped", async () => {
    const wrapper = mountShell(createMockIpc({
      app_get_info: () => shellAppInfo, group_list: () => shellGroups,
      server_appearance_list: () => [],
      server_list: () => ({ items: [...shellServers, { ...shellServers[0]!, id: "orphan", name: "Other", groupId: "removed" }], nextCursor: null }),
    }));
    await flushPromises();
    expect(wrapper.get('[aria-label="未分组"]').text()).toContain("Other");
    await wrapper.get("#shell-global-search").setValue("dev.example");
    expect(wrapper.findAll(".shell-server-item")).toHaveLength(1);
    expect(wrapper.get('.shell-sidebar-search input').element).toHaveProperty("value", "dev.example");
    await wrapper.get('.shell-sidebar-search input').setValue("not-found");
    expect(wrapper.get("nav").text()).toContain("没有找到匹配项");
    expect(wrapper.get("h1").text()).toBe("服务器");
    await wrapper.get('.shell-sidebar-search input').setValue(" ROOT ");
    expect(wrapper.findAll(".shell-server-item")).toHaveLength(4);
  });

  it("shows safe backend errors and supports a real reload on retry", async () => {
    let fail = true;
    const wrapper = mountShell(createMockIpc({
      app_get_info: () => shellAppInfo, group_list: () => [],
      server_appearance_list: () => [],
      server_list: () => {
        if (fail) throw new Error("private database path and debug details");
        return { items: [], nextCursor: null };
      },
    }));
    await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("本地服务暂不可用");
    expect(wrapper.text()).not.toContain("private database");
    expect(wrapper.find(".shell-welcome").exists()).toBe(false);
    fail = false;
    await wrapper.get("main button").trigger("click");
    await flushPromises();
    expect(wrapper.get("h1").text()).toBe("还没有服务器");
    expect(wrapper.get('[role="status"]').text()).toContain("本地服务已就绪");
  });

  it("opens command palette with the platform shortcut and restores About trigger focus", async () => {
    const wrapper = mountShell(createMockIpc({
      app_get_info: () => ({ ...shellAppInfo, platform: "windows" }), group_list: () => [], server_list: () => ({ items: [], nextCursor: null }), server_appearance_list: () => [],
    }));
    await flushPromises();
    expect(wrapper.get("kbd").text()).toBe("Ctrl K");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true, cancelable: true }));
    await flushPromises();
    expect(document.activeElement?.getAttribute("role")).toBe("combobox");
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await flushPromises();
    const trigger = wrapper.get<HTMLButtonElement>('[aria-label="关于 MauLink"]');
    trigger.element.focus();
    await trigger.trigger("click");
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain("0.1.0");
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(trigger.element);
    wrapper.unmount();
    const key = new KeyboardEvent("keydown", { key: "k", ctrlKey: true, cancelable: true });
    document.dispatchEvent(key);
    expect(key.defaultPrevented).toBe(false);
  });
});
