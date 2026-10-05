import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createServerAppearanceApi(client: IpcClient) {
  return {
    list: (payload: Payload<"server_appearance_list"> = {}) => client.call("server_appearance_list", payload),
    get: (payload: Payload<"server_appearance_get">) => client.call("server_appearance_get", payload),
    update: (payload: Payload<"server_appearance_update">) => client.call("server_appearance_update", payload),
  };
}
