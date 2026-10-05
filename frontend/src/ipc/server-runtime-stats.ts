import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createServerRuntimeStatsApi(client: IpcClient) {
  return {
    get: (payload: Payload<"server_runtime_stats_get">) => client.call("server_runtime_stats_get", payload),
  };
}
