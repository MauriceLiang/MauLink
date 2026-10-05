import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createPreflightApi(client: IpcClient) {
  return {
    check: (payload: Payload<"connection_preflight">) => client.call("connection_preflight", payload),
  };
}
