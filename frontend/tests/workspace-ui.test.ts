import { describe, expect, it } from "vitest";
import type { ConnectionSnapshot } from "../../contracts/v1/ConnectionSnapshot";
import { createWorkspaceUiStore } from "../src/stores/workspace-ui";

function snapshot(serverId: string, state: ConnectionSnapshot["state"]): ConnectionSnapshot {
  return {
    connectionId: `connection-${serverId}`,
    serverId,
    mode: "workspace",
    state,
    hostKeyChallenge: null,
    authenticationChallenge: null,
    negotiatedAlgorithms: null,
    error: null,
    createdAtMs: 1,
    updatedAtMs: 1,
  };
}

describe("workspace UI store", () => {
  it("keeps one tab per server and stores pane and connection state", () => {
    const store = createWorkspaceUiStore();
    store.open("web-01");
    store.open("db-01", "files");
    store.open("web-01", "monitor");

    expect(store.tabs.value).toHaveLength(2);
    expect(store.activeServerId.value).toBe("web-01");
    expect(store.activeTab.value?.activePane).toBe("monitor");
    store.syncConnection("web-01", snapshot("web-01", "ready"));
    expect(store.activeTab.value).toMatchObject({ connectionId: "connection-web-01", phase: "open" });
    store.setPane("web-01", "files");
    expect(store.activeTab.value?.activePane).toBe("files");
  });

  it("selects an existing tab and focuses the neighbor when a tab closes", () => {
    const store = createWorkspaceUiStore();
    store.open("web-01");
    store.open("db-01");
    store.open("dev-01");

    expect(store.select("web-01")).toBe(true);
    expect(store.select("missing")).toBe(false);
    expect(store.close("dev-01")?.serverId).toBe("dev-01");
    expect(store.activeServerId.value).toBe("web-01");
    store.select("db-01");
    store.close("db-01");
    expect(store.activeServerId.value).toBe("web-01");
    store.close("web-01");
    expect(store.tabs.value).toEqual([]);
    expect(store.activeServerId.value).toBeNull();
  });

  it("keeps failed and closing phases distinct from a ready SSH connection", () => {
    const store = createWorkspaceUiStore();
    store.open("web-01");
    store.syncConnection("web-01", snapshot("web-01", "connecting"));
    expect(store.activeTab.value?.phase).toBe("opening");
    store.syncConnection("web-01", snapshot("web-01", "ready"));
    expect(store.activeTab.value?.phase).toBe("open");
    store.setPhase("web-01", "closing");
    expect(store.activeTab.value?.phase).toBe("closing");
    store.syncConnection("web-01", snapshot("web-01", "failed"));
    expect(store.activeTab.value?.phase).toBe("failed");
  });
});
