import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createServerApi(client: IpcClient) {
  return {
    list: (payload: Payload<"server_list">) => client.call("server_list", payload),
    get: (payload: Payload<"server_get">) => client.call("server_get", payload),
    create: (payload: Payload<"server_create">) => client.call("server_create", payload),
    update: (payload: Payload<"server_update">) => client.call("server_update", payload),
    remove: (payload: Payload<"server_delete">) => client.call("server_delete", payload),
    listGroups: (payload: Payload<"group_list"> = {}) => client.call("group_list", payload),
    createGroup: (payload: Payload<"group_create">) => client.call("group_create", payload),
    updateGroup: (payload: Payload<"group_update">) => client.call("group_update", payload),
    removeGroup: (payload: Payload<"group_delete">) => client.call("group_delete", payload),
    listRetainedCredentials: (payload: Payload<"credential_list_retained"> = {}) => client.call("credential_list_retained", payload),
    removeRetainedCredential: (payload: Payload<"credential_delete_retained">) => client.call("credential_delete_retained", payload),
    retryCredentialCleanup: (payload: Payload<"credential_cleanup_retry">) => client.call("credential_cleanup_retry", payload),
    selectLocalFile: (payload: Payload<"local_file_select">) => client.call("local_file_select", payload),
  };
}
