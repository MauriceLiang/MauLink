import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createNetworkApi(client: IpcClient) {
  return {
    inspect: (payload: Payload<"network_inspect">) => client.call("network_inspect", payload),
  };
}
