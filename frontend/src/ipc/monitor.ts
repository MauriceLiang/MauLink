import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createMonitorApi(client: IpcClient) {
  return {
    getSnapshot: (payload: Payload<"monitor_get_snapshot">) => client.call("monitor_get_snapshot", payload),
    getHistory: (payload: Payload<"monitor_get_history">) => client.call("monitor_get_history", payload),
    refresh: (payload: Payload<"monitor_refresh">) => client.call("monitor_refresh", payload),
    setActivity: (payload: Payload<"workspace_set_activity">) => client.call("workspace_set_activity", payload),
  };
}
