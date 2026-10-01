import type { ConnectionSnapshot } from "../../../contracts/v1/ConnectionSnapshot";
import type { IpcTransport } from "../ipc/client";
import { createMockIpc } from "../ipc/mock";
import { createServerMock, fixtureError } from "./server-fixtures";
export type ConnectionScenario = "unknown" | "changed" | "password" | "passphrase" | "refused" | "timeout" | "proxy" | "jump" | "expired";

export function createConnectionMock(scenario: () => ConnectionScenario, delay: () => number = () => 0) {
  const servers = createServerMock();
  const snapshots = new Map<string, ConnectionSnapshot>();
  const calls: string[] = [];
  const states = new Map<string, ConnectionScenario>();
  function get(id: string) { const snapshot = snapshots.get(id); if (!snapshot) throw fixtureError("RESOURCE_NOT_FOUND", "errors.connectionNotFound"); return snapshot; }
  function update(id: string, patch: Partial<ConnectionSnapshot>) { const next = { ...get(id), ...patch, updatedAtMs: Date.now() }; snapshots.set(id, next); return structuredClone(next); }
  function auth(id: string) {
    return update(id, { state: "awaitingCredentials", hostKeyChallenge: null, authenticationChallenge: { challengeId: crypto.randomUUID(), connectionId: id, credentialKind: states.get(id) === "passphrase" ? "passphrase" : "password", expiresAtMs: Date.now() + 120000 } });
  }
  const waitForResponse = async () => { const duration = delay(); if (duration) await new Promise(resolve => setTimeout(resolve, duration)); };
  const connection = createMockIpc({
    connection_start: ({ source, mode }) => {
      const id = crypto.randomUUID();
      const choice = scenario(); states.set(id, choice);
      const snapshot: ConnectionSnapshot = { connectionId: id, serverId: source.kind === "saved" ? source.serverId : null, mode, state: "connecting", hostKeyChallenge: null, authenticationChallenge: null, negotiatedAlgorithms: null, error: null, createdAtMs: Date.now(), updatedAtMs: Date.now() };
      snapshots.set(id, snapshot); return structuredClone(snapshot);
    },
    connection_get: ({ connectionId: id }) => {
      const snapshot = get(id); const choice = states.get(id);
      if (snapshot.state !== "connecting") return structuredClone(snapshot);
      if (["refused", "timeout", "proxy", "jump", "expired"].includes(choice!)) {
        const error = fixtureError(choice === "timeout" ? "CONNECTION_TIMEOUT" : choice === "expired" ? "CHALLENGE_EXPIRED" : "CONNECTION_REFUSED", choice === "timeout" ? "errors.connectionTimeout" : choice === "proxy" ? "errors.proxyConnectionFailed" : choice === "expired" ? "errors.challengeExpired" : "errors.connectionRefused");
        error.retryable = choice !== "expired"; error.stage = choice === "proxy" ? "connectingProxy" : "connecting";
        return update(id, { state: "failed", error });
      }
      if (choice === "password" || choice === "passphrase") return auth(id);
      return update(id, { state: "awaitingHostTrust", hostKeyChallenge: { challengeId: crypto.randomUUID(), connectionId: id, host: "fixture.example.test", port: 22, algorithm: "ssh-ed25519", fingerprintSha256: "SHA256:phase5-current-fingerprint", previousFingerprintSha256: choice === "changed" ? "SHA256:phase5-previous-fingerprint" : null, previousRevision: choice === "changed" ? 1 : null, expiresAtMs: Date.now() + 120000 } });
    },
    host_key_respond: async ({ connectionId: id, challengeId, decision }) => {
      await waitForResponse();
      const snapshot = get(id);
      if (snapshot.hostKeyChallenge?.challengeId !== challengeId) throw fixtureError("CHALLENGE_EXPIRED", "errors.challengeExpired");
      if (decision === "reject") update(id, { state: "failed", hostKeyChallenge: null, error: fixtureError(states.get(id) === "changed" ? "HOST_KEY_CHANGED" : "HOST_KEY_REJECTED", states.get(id) === "changed" ? "errors.hostKeyChanged" : "errors.hostKeyRejected") });
      else auth(id);
    },
    auth_respond: async ({ connectionId: id, challengeId, secret }) => {
      await waitForResponse();
      if (get(id).authenticationChallenge?.challengeId !== challengeId) throw fixtureError("CHALLENGE_EXPIRED", "errors.challengeExpired");
      update(id, { state: secret === "wrong" ? "failed" : "ready", authenticationChallenge: null, error: secret === "wrong" ? fixtureError("AUTH_FAILED", "errors.authFailed") : null });
      // Do not retain submitted credentials in the fixture or command log.
    },
    connection_cancel: ({ connectionId: id }) => update(id, { state: "cancelled", hostKeyChallenge: null, authenticationChallenge: null }),
    connection_disconnect: ({ connectionId: id }) => update(id, { state: "closed", hostKeyChallenge: null, authenticationChallenge: null }),
  });
  const transport: IpcTransport = { async invoke<T>(command: string, args?: Record<string, unknown>) {
    calls.push(command);
    return ["connection_start", "connection_get", "connection_cancel", "connection_disconnect", "host_key_respond", "auth_respond"].includes(command) ? connection.invoke<T>(command, args) : servers.invoke<T>(command, args);
  } };
  return { transport, calls };
}
