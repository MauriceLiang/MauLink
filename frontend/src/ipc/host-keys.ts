import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createHostKeysApi(client: IpcClient) {
  return {
    get: (payload: Payload<"host_key_get">) => client.call("host_key_get", payload),
  };
}
