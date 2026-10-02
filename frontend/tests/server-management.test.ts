import { DOMWrapper } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { createIpcClient, type IpcTransport } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createServerApi } from "../src/ipc/server";
import { createServerStore } from "../src/stores/servers";
import { createServerMock, fixtureError } from "../src/harness/server-fixtures";
import { shellServers } from "../src/harness/shell-fixtures";
import { credentialValidation, newServerDraft, normalizeServerDraft, validateServerDraft } from "../src/dialogs/server-form";
import ServerDialog from "../src/dialogs/ServerDialog.vue";
import ConfirmDialog from "../src/dialogs/ConfirmDialog.vue";
import GroupDialog from "../src/dialogs/GroupDialog.vue";
import AppShell from "../src/app/AppShell.vue";
import { startOccupancyProbe } from "../src/harness/occupancy-probe";
import type { ServerMutationResult } from "../../contracts/v1/ServerMutationResult";

const ui = () => new DOMWrapper(document.body);
const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ""; });
const profile = shellServers[0]!;
const makeStore = (transport: IpcTransport = createServerMock()) => createServerStore(createServerApi(createIpcClient(transport)));
function mountEditor(store = makeStore(), serverId: string | null = null) {
  const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId, store } });
  wrappers.push(wrapper);
  return wrapper;
}
async function enterRequired(_wrapper: VueWrapper) {
  await ui().get('input[placeholder="192.168.1.10"]').setValue("test.example.com");
  await ui().get('input[placeholder="root"]').setValue("deploy");
}

describe("server management contracts", () => {
  it("never starts the native occupancy probe for a remote, credentialed or proxied profile", async () => {
    const start = vi.fn();
    const fixture = { ...profile, name: "Phase4-占用验收", host: "127.0.0.1", port: 42424, username: "phase4", connectTimeoutMs: 120000 };
    for (const change of [{ host: "remote.example.com" }, { hasSavedCredential: true }, { proxyType: "socks5" as const }, { jumpHost: "bastion.example.com" }, { port: 22 }]) {
      const unsafe = { ...fixture, ...change };
      const client = createIpcClient(createMockIpc({ server_list: () => ({ items: [unsafe], nextCursor: null }), server_get: () => unsafe, connection_start: start }));
      expect(await startOccupancyProbe(client)).toBeNull();
    }
    expect(start).not.toHaveBeenCalled();
  });

  it("uses the latest revision and test mode only for the guarded loopback occupancy fixture", async () => {
    const fixture = { ...profile, name: "Phase4-占用验收", host: "127.0.0.1", port: 42424, username: "phase4", connectTimeoutMs: 120000, revision: 9 };
    const start = vi.fn(() => { throw fixtureError("SERVER_IN_USE", "errors.serverInUse"); });
    const client = createIpcClient(createMockIpc({ server_list: () => ({ items: [{ ...fixture, revision: 1 }], nextCursor: null }), server_get: () => fixture, connection_start: start }));
    await expect(startOccupancyProbe(client)).rejects.toMatchObject({ code: "SERVER_IN_USE" });
    expect(start).toHaveBeenCalledWith({ source: { kind: "saved", serverId: fixture.id, expectedRevision: 9 }, mode: "test" });
  });

  it("updates local state only after mutation success, uses returned revisions and filters without IPC", async () => {
    const mock = createServerMock({ state: "empty" });
    const store = makeStore(mock);
    await store.load();
    const draft = { ...newServerDraft(), host: "fixture.example.com", username: "deploy" };
    const result = await store.create({ profile: normalizeServerDraft(draft, null), credential: { mode: "replace", secret: "test-only-secret" } });
    expect(store.servers.value[0]?.hasSavedCredential).toBe(true);
    expect(JSON.stringify(store.servers.value)).not.toMatch(/test-only-secret|privateKeyToken/);
    const updated = await store.update({ serverId: result.server.id, expectedRevision: result.server.revision, profile: { ...normalizeServerDraft(draft, null), name: "Renamed" }, credential: { mode: "keep" } });
    expect(updated.server.revision).toBe(2);
    await expect(store.update({ serverId: result.server.id, expectedRevision: 1, profile: normalizeServerDraft(draft, null), credential: { mode: "keep" } })).rejects.toMatchObject({ code: "REVISION_CONFLICT" });
    expect(store.servers.value[0]?.name).toBe("Renamed");
    const count = mock.commands.length;
    store.query.value = " DEPLOY ";
    expect(store.filtered.value).toHaveLength(1);
    store.query.value = "not-found";
    expect(store.filtered.value).toHaveLength(0);
    expect(mock.commands).toHaveLength(count);
    await store.remove({ serverId: updated.server.id, expectedRevision: updated.server.revision, removeCredentials: true });
    expect(store.servers.value).toHaveLength(0);
  });

  it("keeps the last complete list if a later page fails", async () => {
    let failing = false;
    const store = makeStore(createMockIpc({ group_list: () => [], server_list: payload => {
      if (failing && payload.cursor === "next") throw fixtureError("REVISION_CONFLICT", "errors.listCursorExpired");
      return { items: [failing ? { ...profile, name: "Partial data" } : profile], nextCursor: failing ? "next" : null };
    } }));
    expect(await store.load()).toBe(true);
    failing = true;
    expect(await store.load()).toBe(false);
    expect(store.servers.value.map(item => item.name)).toEqual([profile.name]);
    expect(store.error.value?.message).toContain("列表已变化");
  });

  it("validates endpoints, advanced fields, private key lifetime and explicit credential identity changes", () => {
    const draft = { ...newServerDraft(profile), host: "example.com", username: "deploy" };
    expect(validateServerDraft(draft, profile, null)).toBeNull();
    for (const port of [0, 65536, 1.5, Number.NaN]) expect(validateServerDraft({ ...draft, port }, profile, null)).toBe("portInvalid");
    expect(validateServerDraft({ ...draft, host: "https://example.com" }, profile, null)).toBe("hostRequired");
    expect(validateServerDraft({ ...draft, keepaliveIntervalSeconds: 4 }, profile, null)).toBe("keepaliveInvalid");
    expect(validateServerDraft({ ...draft, connectTimeoutMs: 121000 }, profile, null)).toBe("timeoutInvalid");
    expect(validateServerDraft({ ...draft, proxyType: "socks5", proxyHost: "", proxyPort: 1080 }, profile, null)).toBe("proxyInvalid");
    expect(validateServerDraft({ ...draft, authType: "privateKey" }, profile, null)).toBe("keyRequired");
    expect(validateServerDraft({ ...draft, authType: "privateKey" }, null, { token: "fixture", purpose: "privateKey", displayName: "fixture-key", expiresAtMs: Date.now() - 1 })).toBe("keyExpired");
    expect(credentialValidation({ ...profile, hasSavedCredential: true }, draft, { mode: "keep" })).toBe("identityChanged");
    expect(credentialValidation({ ...profile, hasSavedCredential: true }, draft, { mode: "clear" })).toBeNull();
    expect(credentialValidation(profile, draft, { mode: "replace", secret: "" })).toBe("secretRequired");
    expect(normalizeServerDraft({ ...draft, proxyType: null, proxyHost: "stale", proxyPort: 1080 }, null)).toMatchObject({ proxyHost: null, proxyPort: null, privateKeyToken: null });
  });

  it("validates before creating and clears transient credentials when the dialog closes", async () => {
    const create = vi.fn(() => ({ server: profile, credentialCleanupPending: false }));
    const wrapper = mountEditor(makeStore(createMockIpc({ server_create: create })));
    await flushPromises();
    await ui().get("form").trigger("submit");
    expect(create).not.toHaveBeenCalled();
    expect(ui().get('[role="alert"]').text()).toContain("主机地址");
    await enterRequired(wrapper);
    await ui().get('input[type="password"]').setValue("test-transient-secret");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ credential: { mode: "replace", secret: "test-transient-secret" }, profile: expect.objectContaining({ host: "test.example.com", privateKeyToken: null }) }));
    expect(wrapper.emitted("saved")?.[0]).toEqual(["服务器已添加。"]);
    await wrapper.setProps({ open: false });
    await wrapper.setProps({ open: true });
    await flushPromises();
    expect(ui().get<HTMLInputElement>('input[type="password"]').element.value).toBe("");
  });

  it("passes only the selected private key token and preserves an existing key without exposing its path", async () => {
    const create = vi.fn(() => ({ server: { ...profile, authType: "privateKey" as const, hasPrivateKey: true }, credentialCleanupPending: false }));
    const wrapper = mountEditor(makeStore(createMockIpc({ server_create: create, local_file_select: () => ({ token: "opaque-key-token", displayName: "fixture-key", purpose: "privateKey", expiresAtMs: Date.now() + 60000 }) })));
    await flushPromises();
    await enterRequired(wrapper);
    await ui().findAll("button").find(button => button.text() === "SSH 密钥")!.trigger("click");
    await ui().get("form").trigger("submit");
    expect(create).not.toHaveBeenCalled();
    await ui().findAll("button").find(button => button.text() === "选择私钥")!.trigger("click");
    await flushPromises();
    expect(ui().text()).toContain("fixture-key");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ profile: expect.objectContaining({ privateKeyToken: "opaque-key-token" }), credential: { mode: "clear" } }));
    expect(validateServerDraft({ ...newServerDraft({ ...profile, authType: "privateKey", hasPrivateKey: true }) }, { ...profile, authType: "privateKey", hasPrivateKey: true }, null)).toBeNull();
  });

  it("reads the current profile before editing and submits its revision rather than stale list data", async () => {
    const current = { ...profile, revision: 7, hasSavedCredential: true };
    const update = vi.fn(() => ({ server: { ...current, revision: 8 }, credentialCleanupPending: true }));
    const wrapper = mountEditor(makeStore(createMockIpc({ server_get: () => current, server_update: update })), profile.id);
    await flushPromises();
    expect(ui().find('input[type="password"]').exists()).toBe(false);
    await ui().get('input[placeholder="Production Web"]').setValue("Renamed");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(update).toHaveBeenCalledWith(expect.objectContaining({ serverId: profile.id, expectedRevision: 7, credential: { mode: "keep" } }));
    expect(wrapper.emitted("saved")?.[0]?.[0]).toContain("旧凭据清理尚未完成");
  });

  it.each(["inUse", "revision", "unknown"] as const)("keeps the editor open and safely reports %s without changing local profile state", async failure => {
    const store = makeStore(createServerMock({ failure: () => failure }));
    await store.load();
    const wrapper = mountEditor(store, profile.id);
    await flushPromises();
    await ui().get('input[placeholder="Production Web"]').setValue("Attempted change");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(ui().get('[role="alert"]').text()).not.toMatch(/Fixture|database|debug/);
    expect(wrapper.emitted("close")).toBeUndefined();
    expect(store.servers.value[0]?.name).toBe(profile.name);
    if (failure === "inUse") expect(ui().text()).toContain("服务器正在使用中");
    if (failure === "revision") expect(ui().text()).toContain("重新加载（放弃修改）");
  });

  it("prevents duplicate submission and Esc during a pending mutation", async () => {
    let resolve!: (value: ServerMutationResult) => void;
    const promise = new Promise<ServerMutationResult>(done => { resolve = done; });
    const create = vi.fn(() => promise);
    const wrapper = mountEditor(makeStore(createMockIpc({ server_create: create })));
    await flushPromises();
    await enterRequired(wrapper);
    await ui().get("form").trigger("submit");
    await ui().get("form").trigger("submit");
    await ui().get('[role="dialog"], [role="alertdialog"]').trigger("keydown", { key: "Escape" });
    expect(create).toHaveBeenCalledTimes(1);
    expect(ui().get<HTMLFieldSetElement>("fieldset").element.disabled).toBe(true);
    expect(wrapper.emitted("close")).toBeUndefined();
    resolve({ server: profile, credentialCleanupPending: false });
    await flushPromises();
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("deletes only after explicit confirmation with the displayed revision and credential removal", async () => {
    const remove = vi.fn(() => ({ credentialCleanupPending: false }));
    const wrapper = mount(ConfirmDialog, { attachTo: document.body, props: { server: { ...profile, revision: 4 }, store: makeStore(createMockIpc({ server_delete: remove })) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(remove).not.toHaveBeenCalled();
    await ui().get('button.base-button--danger').trigger("click");
    await flushPromises();
    expect(remove).toHaveBeenCalledWith({ serverId: profile.id, expectedRevision: 4, removeCredentials: true });
    expect(wrapper.emitted("removed")?.[0]).toEqual(["服务器与已保存凭据已删除。"]);
  });

  it("returns focus to Home after successful deletion removes the original trigger", async () => {
    const wrapper = mount(AppShell, { attachTo: document.body, props: { client: createIpcClient(createServerMock()) } });
    wrappers.push(wrapper);
    await flushPromises();
    await ui().get('[aria-label="Web-01 · 192.168.1.20"]').trigger("click");
    const trigger = ui().get("main").findAll("button").find(button => button.text() === "删除服务器")!;
    trigger.element.focus();
    await trigger.trigger("click");
    await flushPromises();
    await ui().get(".base-button--danger").trigger("click");
    await flushPromises();
    expect(ui().find('[role="dialog"], [role="alertdialog"]').exists()).toBe(false);
    expect(document.activeElement).toBe(ui().get(".shell-brand").element);
  });

  it("supports group create/rename and confirms deletion before moving members to ungrouped", async () => {
    const mock = createServerMock();
    const store = makeStore(mock);
    await store.load();
    const wrapper = mount(GroupDialog, { attachTo: document.body, props: { open: true, store } });
    wrappers.push(wrapper);
    await flushPromises();
    await ui().get("input").setValue("QA");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(store.groups.value.map(group => group.name)).toContain("QA");
    await ui().get('[aria-label="重命名 Production"]').trigger("click");
    await ui().get("input").setValue("Production renamed");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(store.groups.value.find(group => group.id === "production")?.revision).toBe(2);
    await ui().get('[aria-label="删除分组 Production renamed"]').trigger("click");
    expect(mock.commands).not.toContain("group_delete");
    expect(ui().text()).toContain("删除分组不会删除服务器");
    await ui().get('button.base-button--danger').trigger("click");
    await flushPromises();
    expect(store.servers.value).toHaveLength(3);
    expect(store.servers.value.filter(server => server.groupId === null)).toHaveLength(2);
    expect(document.activeElement).toBe(ui().get("input").element);
  });
});
