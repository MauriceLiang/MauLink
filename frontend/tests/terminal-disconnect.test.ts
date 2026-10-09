import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { ref } from "vue";
import type { ConnectionSnapshot } from "../../contracts/v1/ConnectionSnapshot";
import type { MonitorStore } from "../src/stores/monitor";
import type { TerminalController } from "../src/terminal/controller";
import type { TerminalPreferences } from "../src/terminal/preferences";
import type { TransferStore } from "../src/stores/transfers";
import type { createSftpApi } from "../src/ipc/sftp";
import type { BackgroundImagesApi } from "../src/ipc/background-images";
import TerminalWorkspace from "../src/components/terminal/TerminalWorkspace.vue";
import { shellServers } from "../src/harness/shell-fixtures";
import { visualTransfers } from "../src/harness/visual-fixtures";
import { defaultSettings } from "../src/terminal/preferences";

const wrappers: VueWrapper[] = [];
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = "";
});

function mountWorkspace({ activeTransfer = false, confirmBeforeDisconnect = true } = {}) {
  const connectionId = "fixture-connection";
  const preferences = {
    record: ref({ value: { ...defaultSettings, confirmBeforeDisconnect }, revision: 1, updatedAtMs: 1 }),
    busy: ref(false), error: ref(""), copyOnSelect: ref(false), load: vi.fn(async () => undefined), save: vi.fn(async () => true),
  } as unknown as TerminalPreferences;
  const controller = {
    tabs: ref([]), create: vi.fn(() => "fixture-terminal"), focus: vi.fn(), close: vi.fn(async () => true),
    clear: vi.fn(), refreshConnection: vi.fn(async () => undefined), applySettings: vi.fn(), setCopyPreference: vi.fn(),
  } as unknown as TerminalController;
  const transfers = {
    snapshots: ref(activeTransfer ? visualTransfers(connectionId) : []), starting: ref({}),
  } as unknown as TransferStore;
  const monitor = {
    snapshot: ref(null), histories: ref({}), error: ref(null), activityError: ref(null), historyErrors: ref({}),
    pending: ref(false), refreshing: ref(false), refresh: vi.fn(async () => true),
  } as unknown as MonitorStore;
  const snapshot: ConnectionSnapshot = {
    connectionId, serverId: shellServers[0]!.id, mode: "workspace", state: "ready", hostKeyChallenge: null,
    authenticationChallenge: null, negotiatedAlgorithms: null, error: null, createdAtMs: 1, updatedAtMs: 1,
  };
  const wrapper = mount(TerminalWorkspace, {
    attachTo: document.body,
    props: {
      server: shellServers[0]!, snapshot, controller, sftp: {} as ReturnType<typeof createSftpApi>, transfers, monitor,
      preferences, backgroundImages: {} as BackgroundImagesApi, visible: false, busy: false,
    },
  });
  wrappers.push(wrapper);
  return wrapper;
}

describe("SSH disconnect confirmation", () => {
  it("requires explicit transfer stop consent and emits the selected value once", async () => {
    const wrapper = mountWorkspace({ activeTransfer: true });
    await flushPromises();
    await wrapper.get('[aria-label="断开连接"]').trigger("click");
    await flushPromises();

    const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
    expect(dialog?.getAttribute("data-dialog-size")).toBe("standard");
    expect(dialog?.textContent).toContain("Web-01");
    expect(dialog?.textContent).toContain("停止该连接的传输任务后断开");
    const confirm = Array.from(dialog?.querySelectorAll<HTMLButtonElement>("button") ?? []).find(button => button.textContent?.includes("确认断开"));
    expect(confirm?.disabled).toBe(true);
    expect(wrapper.emitted("disconnect")).toBeUndefined();

    document.querySelector<HTMLElement>('[role="checkbox"]')?.click();
    await flushPromises();
    expect(confirm?.disabled).toBe(false);
    document.querySelector<HTMLButtonElement>('button.base-button--danger')?.click();
    expect(wrapper.emitted("disconnect")).toEqual([[true]]);
  });

  it("disconnects directly when confirmation is disabled and there are no active transfers", async () => {
    const wrapper = mountWorkspace({ confirmBeforeDisconnect: false });
    await flushPromises();
    await wrapper.get('[aria-label="断开连接"]').trigger("click");
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(wrapper.emitted("disconnect")).toEqual([[false]]);
  });
});
