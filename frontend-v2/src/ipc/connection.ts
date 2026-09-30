import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createConnectionApi(client: IpcClient) {
  return {
    start: (payload: Payload<"connection_start">) => client.call("connection_start", payload),
    get: (payload: Payload<"connection_get">) => client.call("connection_get", payload),
    cancel: (payload: Payload<"connection_cancel">) => client.call("connection_cancel", payload),
    respondHostKey: (payload: Payload<"host_key_respond">) => client.call("host_key_respond", payload),
    respondAuthentication: (payload: Payload<"auth_respond">) => client.call("auth_respond", payload),
    disconnect: (payload: Payload<"connection_disconnect">) => client.call("connection_disconnect", payload),
  };
}
