import { computed, ref } from "vue";
import type { ConnectionSnapshot } from "../../../contracts/v1/ConnectionSnapshot";

export type WorkspacePane = "terminal" | "files" | "monitor";
export type WorkspacePhase = "opening" | "open" | "closing" | "failed";
export interface WorkspaceUiTab {
  key: string;
  serverId: string;
  connectionId: string | null;
  activePane: WorkspacePane;
  phase: WorkspacePhase;
}

function snapshotPhase(snapshot: ConnectionSnapshot | null, hasError: boolean): WorkspacePhase {
  if (snapshot?.state === "ready") return "open";
  if (snapshot?.state === "disconnecting") return "closing";
  if (hasError || snapshot?.state === "failed" || snapshot?.state === "closed" || snapshot?.state === "cancelled") return "failed";
  return "opening";
}

export function createWorkspaceUiStore() {
  const tabs = ref<WorkspaceUiTab[]>([]);
  const activeServerId = ref<string | null>(null);
  const activeTab = computed(() => tabs.value.find(tab => tab.serverId === activeServerId.value) ?? null);

  function open(serverId: string, pane: WorkspacePane = "terminal") {
    const existing = tabs.value.find(tab => tab.serverId === serverId);
    if (existing) {
      tabs.value = tabs.value.map(tab => tab.serverId === serverId ? { ...tab, activePane: pane } : tab);
      activeServerId.value = serverId;
      return tabs.value.find(tab => tab.serverId === serverId)!;
    }
    const tab: WorkspaceUiTab = { key: serverId, serverId, connectionId: null, activePane: pane, phase: "opening" };
    tabs.value = [...tabs.value, tab];
    activeServerId.value = serverId;
    return tab;
  }

  function select(serverId: string) {
    if (!tabs.value.some(tab => tab.serverId === serverId)) return false;
    activeServerId.value = serverId;
    return true;
  }

  function setPane(serverId: string, activePane: WorkspacePane) {
    tabs.value = tabs.value.map(tab => tab.serverId === serverId ? { ...tab, activePane } : tab);
  }

  function syncConnection(serverId: string, snapshot: ConnectionSnapshot | null, hasError = false) {
    tabs.value = tabs.value.map(tab => tab.serverId === serverId
      ? { ...tab, connectionId: snapshot?.connectionId ?? null, phase: snapshotPhase(snapshot, hasError) }
      : tab);
  }

  function setPhase(serverId: string, phase: WorkspacePhase) {
    tabs.value = tabs.value.map(tab => tab.serverId === serverId ? { ...tab, phase } : tab);
  }

  function close(serverId: string) {
    const index = tabs.value.findIndex(tab => tab.serverId === serverId);
    if (index < 0) return null;
    const [removed] = tabs.value.splice(index, 1);
    if (activeServerId.value === serverId) {
      activeServerId.value = tabs.value[index - 1]?.serverId ?? tabs.value[0]?.serverId ?? null;
    }
    return removed ?? null;
  }

  return { tabs, activeServerId, activeTab, open, select, setPane, syncConnection, setPhase, close };
}

export type WorkspaceUiStore = ReturnType<typeof createWorkspaceUiStore>;
