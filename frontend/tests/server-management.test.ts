import { DOMWrapper } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { ref } from "vue";
import { createIpcClient, type IpcTransport } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createServerApi } from "../src/ipc/server";
import { createServerStore } from "../src/stores/servers";
import { createServerAppearanceStore } from "../src/stores/server-appearance";
import { createServerAppearanceApi } from "../src/ipc/server-appearance";
import { createBackgroundImagesApi } from "../src/ipc/background-images";
import type { ConnectionStore } from "../src/stores/connections";
import { createServerMock, fixtureError } from "../src/harness/server-fixtures";
import { shellServers } from "../src/harness/shell-fixtures";
import { credentialValidation, newServerDraft, normalizeServerDraft, validateServerDraft } from "../src/dialogs/server-form";
import ServerDialog from "../src/dialogs/ServerDialog.vue";
import ConfirmDialog from "../src/dialogs/ConfirmDialog.vue";
import GroupDialog from "../src/dialogs/GroupDialog.vue";
import AppShell from "../src/app/AppShell.vue";
import { startOccupancyProbe } from "../src/harness/occupancy-probe";
import type { ServerMutationResult } from "../../contracts/v1/ServerMutationResult";
import type { ServerUpdatePayload } from "../../contracts/v1/ServerUpdatePayload";
import type { ServerAppearanceUpdate } from "../../contracts/v1/ServerAppearanceUpdate";
import type { BackgroundImagesApi } from "../src/ipc/background-images";

const ui = () => new DOMWrapper(document.body);
const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ""; });
const profile = shellServers[0]!;
const makeStore = (transport: IpcTransport = createServerMock()) => createServerStore(createServerApi(createIpcClient(transport)));
function mountEditor(store = makeStore(), serverId: string | null = null, connectionStore?: ConnectionStore) {
  const client = createIpcClient(createMockIpc({ server_get: () => profile, server_appearance_get: ({ serverId }) => ({ serverId, labelColor: null, environment: null, terminalOverrideEnabled: false, terminalAppearance: { themeMode: "followApp", customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" }, selectionOpacity: 18, backgroundImage: { imageId: null, fit: "cover", position: "center", imageOpacity: 100, overlayKind: "dark", overlayOpacity: 45, blurPx: 0 } }, revision: 0, updatedAtMs: 0 }) }));
  const appearanceStore = createServerAppearanceStore(createServerAppearanceApi(client));
  const backgroundImages = createBackgroundImagesApi(client, path => path);
  const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId, store, appearanceStore, backgroundImages, connectionStore } });
  wrappers.push(wrapper);
  return wrapper;
}
async function enterRequired(_wrapper: VueWrapper) {
  await ui().get('input[placeholder="192.168.1.10"]').setValue("test.example.com");
  await ui().get('input[placeholder="root"]').setValue("deploy");
}
async function selectDialogTab(section: "basic" | "advanced" | "appearance") {
  const tabs = ui().findAll('[role="tab"]');
  const index = { basic: 0, advanced: 1, appearance: 2 }[section];
  await tabs[index]!.trigger("mousedown", { button: 0, ctrlKey: false });
  await flushPromises();
}
async function chooseCredentialStorage(label: string) {
  const trigger = ui().get(".server-credential-storage-field [role=combobox]");
  trigger.element.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
  await flushPromises();
  const option = ui().findAll('[role="option"]').find(item => item.text().includes(label));
  expect(option).toBeDefined();
  (option!.element as HTMLElement).focus();
  option!.element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  await flushPromises();
}

describe("server management contracts", () => {
  it("saves per-server appearance independently with its own revision", async () => {
    const initialAppearance = {
      serverId: profile.id, labelColor: null, environment: null, terminalOverrideEnabled: false,
      terminalAppearance: {
        themeMode: "followApp" as const,
        customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" },
        selectionOpacity: 18,
        backgroundImage: { imageId: null, fit: "cover" as const, position: "center" as const, imageOpacity: 100, overlayKind: "dark" as const, overlayOpacity: 45, blurPx: 0 },
      }, revision: 0, updatedAtMs: 0,
    };
    const updateProfile = vi.fn();
    const updatedAppearance = vi.fn((payload: ServerAppearanceUpdate) => ({ ...initialAppearance, ...payload, revision: 1, updatedAtMs: 1 }));
    const client = createIpcClient(createMockIpc({
      server_get: () => profile,
      server_update: updateProfile,
      server_appearance_get: () => initialAppearance,
      server_appearance_update: updatedAppearance,
    }));
    const store = createServerStore(createServerApi(client));
    const appearanceStore = createServerAppearanceStore(createServerAppearanceApi(client));
    const backgroundImages = createBackgroundImagesApi(client, path => path);
    const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId: profile.id, store, appearanceStore, backgroundImages } });
    wrappers.push(wrapper);
    await flushPromises();

    const appearanceTab = ui().findAll('[role="tab"]')[2]!;
    await appearanceTab.trigger("mousedown", { button: 0, ctrlKey: false });
    await flushPromises();
    await ui().get('button[aria-label="自定义颜色 #D92D20"]').trigger("click");
    await ui().get('button[role="switch"]').trigger("click");
    await ui().findAll("button").find(button => button.text().trim() === "保存")!.trigger("click");
    await flushPromises();
    expect(updateProfile).not.toHaveBeenCalled();
    expect(updatedAppearance).toHaveBeenCalledWith(expect.objectContaining({
      serverId: profile.id, expectedRevision: 0, labelColor: "#D92D20", terminalOverrideEnabled: true,
    }));
    expect(appearanceStore.appearances.value[profile.id]?.revision).toBe(1);
  });

  it("saves an environment edited in Advanced through the shared footer action", async () => {
    const initialAppearance = {
      serverId: profile.id, labelColor: null, environment: null, terminalOverrideEnabled: false,
      terminalAppearance: {
        themeMode: "followApp" as const,
        customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" },
        selectionOpacity: 18,
        backgroundImage: { imageId: null, fit: "cover" as const, position: "center" as const, imageOpacity: 100, overlayKind: "dark" as const, overlayOpacity: 45, blurPx: 0 },
      }, revision: 0, updatedAtMs: 0,
    };
    const updateProfile = vi.fn();
    const updateAppearance = vi.fn((payload: ServerAppearanceUpdate) => ({ ...initialAppearance, ...payload, revision: 1, updatedAtMs: 1 }));
    const client = createIpcClient(createMockIpc({ server_get: () => profile, server_update: updateProfile, server_appearance_get: () => initialAppearance, server_appearance_update: updateAppearance }));
    const store = createServerStore(createServerApi(client));
    const appearanceStore = createServerAppearanceStore(createServerAppearanceApi(client));
    const backgroundImages = createBackgroundImagesApi(client, path => path);
    const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId: profile.id, store, appearanceStore, backgroundImages } });
    wrappers.push(wrapper);
    await flushPromises();

    await selectDialogTab("advanced");
    const environment = ui().findAll(".base-field").find(field => field.text().includes("环境"))!;
    environment.get("[role=combobox]").element.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
    await flushPromises();
    const staging = ui().findAll('[role="option"]').find(option => option.text() === "预发布环境")!;
    staging.element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await flushPromises();
    await selectDialogTab("basic");
    await ui().findAll("button").find(button => button.text().trim() === "保存")!.trigger("click");
    await flushPromises();

    expect(updateProfile).not.toHaveBeenCalled();
    expect(updateAppearance).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({
      serverId: profile.id, expectedRevision: 0, environment: "staging",
    }));
  });

  it("commits Profile once before Appearance and retries only the failed Appearance stage", async () => {
    const initialAppearance = {
      serverId: profile.id, labelColor: null, environment: null, terminalOverrideEnabled: false,
      terminalAppearance: {
        themeMode: "followApp" as const,
        customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" },
        selectionOpacity: 18,
        backgroundImage: { imageId: null, fit: "cover" as const, position: "center" as const, imageOpacity: 100, overlayKind: "dark" as const, overlayOpacity: 45, blurPx: 0 },
      }, revision: 0, updatedAtMs: 0,
    };
    const order: string[] = [];
    const updateProfile = vi.fn((payload: ServerUpdatePayload) => {
      order.push("profile");
      return { server: { ...profile, name: payload.profile.name || profile.name, id: profile.id, revision: profile.revision + 1 }, credentialCleanupPending: false };
    });
    let failAppearance = true;
    const updateAppearance = vi.fn((payload: ServerAppearanceUpdate) => {
      order.push("appearance");
      if (failAppearance) throw fixtureError("INTERNAL", "errors.internal");
      return { ...initialAppearance, ...payload, revision: 1, updatedAtMs: 1 };
    });
    const client = createIpcClient(createMockIpc({ server_get: () => profile, server_update: updateProfile, server_appearance_get: () => initialAppearance, server_appearance_update: updateAppearance }));
    const store = createServerStore(createServerApi(client));
    const appearanceStore = createServerAppearanceStore(createServerAppearanceApi(client));
    const backgroundImages = createBackgroundImagesApi(client, path => path);
    const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId: profile.id, store, appearanceStore, backgroundImages } });
    wrappers.push(wrapper);
    await flushPromises();

    await ui().get('input[placeholder="Production Web"]').setValue("Production Renamed");
    await selectDialogTab("appearance");
    await ui().get('button[aria-label="自定义颜色 #D92D20"]').trigger("click");
    await ui().findAll("button").find(button => button.text().trim() === "保存")!.trigger("click");
    await flushPromises();

    expect(order).toEqual(["profile", "appearance"]);
    expect(updateProfile).toHaveBeenCalledTimes(1);
    expect(updateAppearance).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted("close")).toBeUndefined();
    expect(ui().text()).toContain("服务器信息已保存，但外观未保存");

    failAppearance = false;
    await ui().findAll("button").find(button => button.text().trim() === "保存")!.trigger("click");
    await flushPromises();
    expect(order).toEqual(["profile", "appearance", "appearance"]);
    expect(updateProfile).toHaveBeenCalledTimes(1);
    expect(updateAppearance.mock.calls[1]![0].expectedRevision).toBe(0);
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("blocks saving Image mode until a background image is selected", async () => {
    const updateProfile = vi.fn();
    const updateAppearance = vi.fn();
    const wrapper = mountEditor(makeStore(createMockIpc({
      server_get: () => profile,
      server_update: updateProfile,
      server_appearance_update: updateAppearance,
    })), profile.id);
    await flushPromises();
    await selectDialogTab("appearance");
    await ui().get('[role="switch"]').trigger("click");
    await ui().findAll(".terminal-theme-mode-option").find(option => option.text().includes("背景图片"))!.trigger("click");
    await ui().findAll("button").find(button => button.text().trim() === "保存")!.trigger("click");
    await flushPromises();

    expect(ui().text()).toContain("请选择背景图片后再保存图片主题。");
    expect(updateProfile).not.toHaveBeenCalled();
    expect(updateAppearance).not.toHaveBeenCalled();
    expect(wrapper.emitted("close")).toBeUndefined();
  });

  it("cleans up an image returned after the editor session has closed", async () => {
    type SelectedAsset = Awaited<ReturnType<BackgroundImagesApi["select"]>>;
    let completeSelection!: (asset: SelectedAsset) => void;
    const deleteImage = vi.fn().mockResolvedValue(undefined);
    const backgroundImages = {
      select: vi.fn(() => new Promise<SelectedAsset>(resolve => { completeSelection = resolve; })),
      get: vi.fn(),
      delete: deleteImage,
      resolve: vi.fn(),
    } as unknown as BackgroundImagesApi;
    const client = createIpcClient(createMockIpc({ server_get: () => profile }));
    const store = createServerStore(createServerApi(client));
    const appearanceStore = createServerAppearanceStore(createServerAppearanceApi(client));
    const wrapper = mount(ServerDialog, { attachTo: document.body, props: { open: true, serverId: profile.id, store, appearanceStore, backgroundImages } });
    wrappers.push(wrapper);
    await flushPromises();
    await selectDialogTab("appearance");
    await ui().get('[role="switch"]').trigger("click");
    await ui().findAll(".terminal-theme-mode-option").find(option => option.text().includes("背景图片"))!.trigger("click");
    await ui().findAll("button").find(button => button.text().trim() === "选择图片")!.trigger("click");

    await wrapper.setProps({ open: false });
    completeSelection({ id: "late-import", fileName: "late.png", mediaType: "image/png", width: 10, height: 10, byteLength: 100, createdAtMs: 1 });
    await flushPromises();

    expect(deleteImage).toHaveBeenCalledExactlyOnceWith("late-import");
    expect(wrapper.emitted("saved")).toBeUndefined();
  });

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

  it("defaults new servers to encrypted local storage and carries each server's storage choice", () => {
    expect(newServerDraft().requireAuthentication).toBe(false);
    expect(newServerDraft({ ...profile, requireAuthentication: true }).requireAuthentication).toBe(true);
  });

  it("validates before creating and clears transient credentials when the dialog closes", async () => {
    const create = vi.fn(() => ({ server: profile, credentialCleanupPending: false }));
    const wrapper = mountEditor(makeStore(createMockIpc({ server_create: create })));
    await flushPromises();
    await ui().get("form").trigger("submit");
    expect(create).not.toHaveBeenCalled();
    expect(ui().get('[role="alert"]').text()).toContain("主机地址");
    await enterRequired(wrapper);
    expect(ui().get(".server-credential-storage-field [role=combobox]").text()).toContain("本地加密存储");
    await chooseCredentialStorage("系统凭据库");
    await ui().get('input[type="password"]').setValue("test-transient-secret");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ credential: { mode: "replace", secret: "test-transient-secret" }, profile: expect.objectContaining({ host: "test.example.com", requireAuthentication: true, privateKeyToken: null }) }));
    expect(wrapper.emitted("saved")?.[0]).toEqual(["服务器已添加。"]);
    await wrapper.setProps({ open: false });
    await wrapper.setProps({ open: true });
    await flushPromises();
    expect(ui().get<HTMLInputElement>('input[type="password"]').element.value).toBe("");
  });

  it("keeps basic and advanced values when switching tabs and opens on Basic for each session", async () => {
    const wrapper = mountEditor();
    await flushPromises();
    wrappers.push(wrapper);
    expect(ui().findAll('[role="tab"]')[0]?.attributes("aria-selected")).toBe("true");
    await ui().get('input[placeholder="192.168.1.10"]').setValue("test.example.com");
    await ui().get('input[type="password"]').setValue("transient-secret");
    await selectDialogTab("advanced");
    await ui().get('input[placeholder="user@bastion.example.com"]').setValue("deploy@bastion.example.com");
    await selectDialogTab("basic");
    expect(ui().get<HTMLInputElement>('input[placeholder="192.168.1.10"]').element.value).toBe("test.example.com");
    expect(ui().get<HTMLInputElement>('input[type="password"]').element.value).toBe("transient-secret");
    await selectDialogTab("advanced");
    expect(ui().get<HTMLInputElement>('input[placeholder="user@bastion.example.com"]').element.value).toBe("deploy@bastion.example.com");
    await wrapper.setProps({ open: false });
    await wrapper.setProps({ open: true });
    await flushPromises();
    expect(ui().findAll('[role="tab"]')[0]?.attributes("aria-selected")).toBe("true");
    expect(ui().get<HTMLInputElement>('input[placeholder="192.168.1.10"]').element.value).toBe("");
  });

  it("routes validation to the tab containing the invalid field, including the two port fields", async () => {
    const wrapper = mountEditor();
    await flushPromises();
    wrappers.push(wrapper);
    await enterRequired(wrapper);
    await selectDialogTab("advanced");
    const keepalive = ui().findAll(".base-field").find(field => field.text().includes("保活间隔"))!;
    await keepalive.get("input").setValue("4");
    await selectDialogTab("basic");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(ui().findAll('[role="tab"]')[1]?.attributes("aria-selected")).toBe("true");
    expect(ui().get('[role="alert"]').text()).toContain("5 到 300");

    const jumpPort = ui().get(".server-advanced-row input[type=number]");
    await jumpPort.setValue("0");
    await selectDialogTab("basic");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(ui().findAll('[role="tab"]')[1]?.attributes("aria-selected")).toBe("true");

    await ui().get(".server-advanced-row input[type=number]").setValue("22");
    await selectDialogTab("basic");
    await ui().get(".server-form-grid input[type=number]").setValue("0");
    await selectDialogTab("advanced");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(ui().findAll('[role="tab"]')[0]?.attributes("aria-selected")).toBe("true");
    expect(ui().get('[role="alert"]').text()).toContain("1 到 65535");
  });

  it("passes only the selected private key token and preserves an existing key without exposing its path", async () => {
    const create = vi.fn(() => ({ server: { ...profile, authType: "privateKey" as const, hasPrivateKey: true }, credentialCleanupPending: false }));
    const wrapper = mountEditor(makeStore(createMockIpc({ server_create: create, local_file_select: () => ({ token: "opaque-key-token", displayName: "fixture-key", purpose: "privateKey", expiresAtMs: Date.now() + 60000 }) })));
    await flushPromises();
    await enterRequired(wrapper);
    await ui().findAll("button").find(button => button.text() === "SSH 密钥")!.trigger("click");
    await ui().get("form").trigger("submit");
    expect(create).not.toHaveBeenCalled();
    await ui().get(".server-key-browse-button").trigger("click");
    await flushPromises();
    expect(ui().text()).toContain("fixture-key");
    expect(ui().text()).not.toContain("opaque-key-token");
    expect(ui().get(".server-key-file-hint").text()).toContain("保存配置后应用");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ profile: expect.objectContaining({ privateKeyToken: "opaque-key-token" }), credential: { mode: "clear" } }));
    expect(validateServerDraft({ ...newServerDraft({ ...profile, authType: "privateKey", hasPrivateKey: true }) }, { ...profile, authType: "privateKey", hasPrivateKey: true }, null)).toBeNull();
  });

  it("keeps the selected private key when the picker is cancelled", async () => {
    const selection = vi.fn()
      .mockReturnValueOnce({ token: "opaque-key-token", displayName: "fixture_ed25519", purpose: "privateKey", expiresAtMs: Date.now() + 60000 })
      .mockReturnValueOnce(null);
    const wrapper = mountEditor(makeStore(createMockIpc({ local_file_select: selection })));
    await flushPromises();
    await enterRequired(wrapper);
    await ui().findAll("button").find(button => button.text() === "SSH 密钥")!.trigger("click");
    await ui().get(".server-key-browse-button").trigger("click");
    await flushPromises();
    await ui().get(".server-key-browse-button").trigger("click");
    await flushPromises();
    expect(ui().get(".server-key-file-name").text()).toBe("fixture_ed25519");
    expect(ui().get(".server-key-file-hint").text()).toContain("保存配置后应用");
  });

  it("marks an expired private key reference and blocks saving it", async () => {
    const expiredCreate = vi.fn();
    const expired = mountEditor(makeStore(createMockIpc({
      server_create: expiredCreate,
      local_file_select: () => ({ token: "expired-key-token", displayName: "expired_ed25519", purpose: "privateKey", expiresAtMs: Date.now() - 1 }),
    })));
    await flushPromises();
    await enterRequired(expired);
    await ui().findAll("button").find(button => button.text() === "SSH 密钥")!.trigger("click");
    await ui().get(".server-key-browse-button").trigger("click");
    await flushPromises();
    expect(ui().get(".server-key-picker-input").attributes("data-state")).toBe("expired");
    expect(ui().get(".server-key-file-hint").text()).toContain("已过期");
    await ui().get("form").trigger("submit");
    expect(expiredCreate).not.toHaveBeenCalled();
    expect(ui().get('[role="alert"]').text()).toContain("过期");
  });

  it("shows an existing private key without inventing its filename or exposing a path", async () => {
    const currentKey = { ...profile, authType: "privateKey" as const, hasPrivateKey: true, hasSavedCredential: true };
    mountEditor(makeStore(createMockIpc({ server_get: () => currentKey })), profile.id);
    await flushPromises();
    expect(ui().get(".server-key-file-name").text()).toBe("已配置私钥文件");
    expect(ui().get(".server-key-file-hint").text()).toContain("沿用当前私钥配置");
    expect(ui().text()).not.toMatch(/fixture_ed25519|\/Users\/|\.pem/);
    expect(ui().find('input[type="password"]').exists()).toBe(false);
    expect(ui().get(".server-credential-storage-summary").text()).toBe("本地加密存储");
    expect(ui().find(".server-credential-storage-field [role=combobox]").exists()).toBe(false);
  });

  it("maps the saved credential storage choice back to the profile boolean when editing", async () => {
    const current = { ...profile, requireAuthentication: true, hasSavedCredential: true };
    const update = vi.fn(() => ({ server: { ...current, requireAuthentication: false, revision: current.revision + 1 }, credentialCleanupPending: false }));
    mountEditor(makeStore(createMockIpc({ server_get: () => current, server_update: update })), profile.id);
    await flushPromises();
    expect(ui().get(".server-credential-storage-summary").text()).toBe("系统凭据库");
    await ui().get('button[aria-label="更多凭据操作"]').trigger("click");
    await flushPromises();
    const changeStorage = ui().findAll('[role="menuitem"]').find(item => item.text() === "更改存储位置");
    expect(changeStorage).toBeDefined();
    await changeStorage!.trigger("click");
    await flushPromises();
    await chooseCredentialStorage("本地加密存储");
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(update).toHaveBeenCalledWith(expect.objectContaining({ profile: expect.objectContaining({ requireAuthentication: false }), credential: { mode: "keep" } }));
  });

  it("shows an existing credential without prompting and discards a cancelled replacement", async () => {
    const current = { ...profile, hasSavedCredential: true };
    const update = vi.fn(() => ({ server: { ...current, revision: current.revision + 1 }, credentialCleanupPending: false }));
    mountEditor(makeStore(createMockIpc({ server_get: () => current, server_update: update })), profile.id);
    await flushPromises();

    expect(ui().get(".server-credential-state").text()).toContain("已保存密码");
    expect(ui().find('input[type="password"]').exists()).toBe(false);
    await ui().findAll("button").find(button => button.text().trim() === "更换")!.trigger("click");
    await flushPromises();
    expect((ui().get('input[type="password"]').element as HTMLInputElement).value).toBe("");
    expect(ui().findAll(".server-credential-storage-field [role=combobox]")).toHaveLength(1);
    await ui().get('input[type="password"]').setValue("unsaved-replacement");
    await ui().findAll("button").find(button => button.text().trim() === "取消更换")!.trigger("click");
    await flushPromises();

    expect(ui().find('input[type="password"]').exists()).toBe(false);
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(update).not.toHaveBeenCalled();
    expect(JSON.stringify(update.mock.calls)).not.toContain("unsaved-replacement");
  });

  it("confirms removal in the draft, allows undo, and clears only on the overall save", async () => {
    const current = { ...profile, hasSavedCredential: true };
    const update = vi.fn(() => ({ server: { ...current, hasSavedCredential: false, revision: current.revision + 1 }, credentialCleanupPending: false }));
    mountEditor(makeStore(createMockIpc({ server_get: () => current, server_update: update })), profile.id);
    await flushPromises();

    const openRemoval = async () => {
      await ui().get('button[aria-label="更多凭据操作"]').trigger("click");
      await flushPromises();
      const remove = ui().findAll('[role="menuitem"]').find(item => item.text() === "移除已保存凭据");
      expect(remove).toBeDefined();
      await remove!.trigger("click");
      await flushPromises();
    };
    await openRemoval();
    expect(ui().get('[role="alertdialog"]').text()).toContain("关闭或取消弹窗会放弃此操作");
    await ui().findAll('[role="alertdialog"] button').find(button => button.text().trim() === "取消")!.trigger("click");
    await flushPromises();
    expect(ui().get(".server-credential-state").attributes("data-state")).toBe("unchanged");

    await openRemoval();
    await ui().findAll('[role="alertdialog"] button').find(button => button.text().trim() === "移除已保存凭据")!.trigger("click");
    await flushPromises();
    expect(ui().get(".server-credential-state").attributes("data-state")).toBe("pendingClear");
    expect(update).not.toHaveBeenCalled();
    await ui().findAll("button").find(button => button.text().trim() === "撤销移除")!.trigger("click");
    await flushPromises();
    expect(ui().get(".server-credential-state").attributes("data-state")).toBe("unchanged");
    expect(update).not.toHaveBeenCalled();

    await openRemoval();
    await ui().findAll('[role="alertdialog"] button').find(button => button.text().trim() === "移除已保存凭据")!.trigger("click");
    await flushPromises();
    expect(update).not.toHaveBeenCalled();
    await ui().get("form").trigger("submit");
    await flushPromises();
    expect(update).toHaveBeenCalledWith(expect.objectContaining({ credential: { mode: "clear" } }));
  });

  it("does not pass a saved credential to draft connection tests while removal is pending", async () => {
    const current = { ...profile, hasSavedCredential: true };
    const startDraftTest = vi.fn().mockResolvedValue(null);
    const connectionStore = {
      snapshots: ref({}),
      startDraftTest,
      cancel: vi.fn(),
      dismiss: vi.fn(),
    } as unknown as ConnectionStore;
    mountEditor(makeStore(createMockIpc({ server_get: () => current })), profile.id, connectionStore);
    await flushPromises();

    await ui().get('button[aria-label="更多凭据操作"]').trigger("click");
    await flushPromises();
    await ui().findAll('[role="menuitem"]').find(item => item.text() === "移除已保存凭据")!.trigger("click");
    await flushPromises();
    await ui().findAll('[role="alertdialog"] button').find(button => button.text().trim() === "移除已保存凭据")!.trigger("click");
    await flushPromises();
    await ui().findAll("button").find(button => button.text().trim() === "测试连接")!.trigger("click");
    await flushPromises();

    expect(startDraftTest).toHaveBeenCalledWith(expect.objectContaining({ host: current.host }), null, undefined);
  });

  it("keeps save disabled while a draft connection test is running", async () => {
    let finishTest!: (snapshot: null) => void;
    const startDraftTest = vi.fn(() => new Promise<null>(resolve => { finishTest = resolve; }));
    const connectionStore = {
      snapshots: ref({}), startDraftTest, cancel: vi.fn(), dismiss: vi.fn(),
    } as unknown as ConnectionStore;
    const wrapper = mountEditor(makeStore(), null, connectionStore);
    await flushPromises();
    await enterRequired(wrapper);

    const test = ui().findAll("button").find(button => button.text().trim() === "测试连接");
    await test!.trigger("click");
    await flushPromises();

    expect(test!.attributes("aria-busy")).toBe("true");
    const save = ui().findAll("button").find(button => button.text().trim() === "保存");
    expect((save?.element as HTMLButtonElement).disabled).toBe(true);

    finishTest(null);
    await flushPromises();
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

  it("routes the saved password menu action to the isolated reveal dialog request", async () => {
    const savedProfile = { ...profile, hasSavedCredential: true };
    const store = makeStore(createMockIpc({ server_get: () => savedProfile }));
    const wrapper = mountEditor(store, savedProfile.id);
    await flushPromises();
    expect(ui().text()).toContain("已保存密码");
    await ui().get('button[aria-label="更多凭据操作"]').trigger("click");
    await flushPromises();
    const revealAction = ui().get('[data-action="reveal"]');
    expect(revealAction.text()).toContain("查看已保存密码");
    await revealAction.trigger("click");
    await flushPromises();
    expect(wrapper.emitted("revealCredential")).toEqual([[savedProfile.id]]);
  });

  it("deletes only after explicit confirmation with the displayed revision and credential removal", async () => {
    const remove = vi.fn(() => ({ credentialCleanupPending: false }));
    const wrapper = mount(ConfirmDialog, { attachTo: document.body, props: { server: { ...profile, revision: 4 }, store: makeStore(createMockIpc({ server_delete: remove })) } });
    wrappers.push(wrapper);
    await flushPromises();
    expect(remove).not.toHaveBeenCalled();
    const dialog = ui().get('[role="alertdialog"]');
    expect(dialog.attributes("data-dialog-size")).toBe("compact");
    expect(document.activeElement).toBe(ui().get('[data-dialog-cancel]').element);
    await ui().get('button.base-button--danger').trigger("click");
    await flushPromises();
    expect(remove).toHaveBeenCalledWith({ serverId: profile.id, expectedRevision: 4, removeCredentials: true });
    expect(wrapper.emitted("removed")?.[0]).toEqual(["服务器与已保存凭据已删除。"]);
  });

  it("returns focus to a surviving server after deletion removes the original trigger", async () => {
    const wrapper = mount(AppShell, { attachTo: document.body, props: { client: createIpcClient(createServerMock()) } });
    wrappers.push(wrapper);
    await flushPromises();
    const webServerItem = ui().findAll('[role="treeitem"]').find(item => item.attributes('aria-label')?.startsWith('Web-01 · 192.168.1.20 · '));
    expect(webServerItem).toBeDefined();
    await webServerItem!.trigger("click");
    await webServerItem!.trigger("contextmenu", { button: 2, clientX: 20, clientY: 20 });
    await flushPromises();
    const deleteAction = ui().get('[data-action="remove"]');
    (deleteAction.element as HTMLElement).focus();
    await deleteAction.trigger("click");
    await flushPromises();
    await ui().get(".base-button--danger").trigger("click");
    await flushPromises();
    expect(ui().find('[role="dialog"], [role="alertdialog"]').exists()).toBe(false);
    expect(document.activeElement).toBe(ui().get(".shell-server-item").element);
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
    await ui().get('[aria-label="重命名 Production renamed"]').trigger("click");
    await ui().get("input").setValue("Unsubmitted group draft");
    await ui().get('[aria-label="删除分组 Production renamed"]').trigger("click");
    const confirmation = ui().get('[role="alertdialog"]');
    expect(confirmation.get("h2").text()).toBe("删除分组？");
    expect(confirmation.attributes("data-dialog-size")).toBe("compact");
    expect(mock.commands).not.toContain("group_delete");
    expect(ui().text()).toContain("删除分组不会删除服务器");
    await ui().get('[data-dialog-cancel]').trigger("click");
    await flushPromises();
    expect(ui().get('[role="dialog"]').text()).toContain("管理分组");
    expect((ui().get("input").element as HTMLInputElement).value).toBe("Unsubmitted group draft");
    expect(store.groups.value.find(group => group.id === "production")?.name).toBe("Production renamed");
    await ui().get('[aria-label="删除分组 Production renamed"]').trigger("click");
    await ui().get('button.base-button--danger').trigger("click");
    await flushPromises();
    expect(store.servers.value).toHaveLength(3);
    expect(store.servers.value.filter(server => server.groupId === null)).toHaveLength(2);
    expect(document.activeElement).toBe(ui().get("input").element);
  });
});
