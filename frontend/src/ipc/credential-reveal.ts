import type { IpcClient } from "./client";
import type { Payload } from "./commands";

export function createCredentialRevealApi(client: IpcClient) {
  return {
    getPolicy: () => client.call("reveal_policy_get", {}),
    enableProtected: (payload: Payload<"reveal_policy_enable_protected">) =>
      client.call("reveal_policy_enable_protected", payload),
    enableDirect: (payload: Payload<"reveal_policy_enable_direct">) =>
      client.call("reveal_policy_enable_direct", payload),
    setDeny: (payload: Payload<"reveal_policy_set_deny">) =>
      client.call("reveal_policy_set_deny", payload),
    changePassword: (payload: Payload<"reveal_policy_change_password">) =>
      client.call("reveal_policy_change_password", payload),
    recover: (payload: Payload<"reveal_policy_recover">) =>
      client.call("reveal_policy_recover", payload),
    reveal: (payload: Payload<"credential_reveal">) =>
      client.call("credential_reveal", payload),
  };
}

export type CredentialRevealApi = ReturnType<typeof createCredentialRevealApi>;
