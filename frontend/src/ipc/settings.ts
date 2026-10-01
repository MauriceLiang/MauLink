import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createSettingsApi(client: IpcClient) {
  return {
    get: (payload: Payload<"settings_get"> = {}) => client.call("settings_get", payload),
    update: (payload: Payload<"settings_update">) => client.call("settings_update", payload),
  };
}
