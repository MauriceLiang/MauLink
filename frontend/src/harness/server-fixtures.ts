import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import { createMockIpc } from "../ipc/mock";
import { shellAppInfo, shellGroups, shellServers } from "./shell-fixtures";
import { networkFixture, preflightFixture } from "./network-fixtures";

export function fixtureError(code: AppError["code"], messageKey: string): AppError {
  return { code, messageKey, params: {}, retryable: false, action: code === "REVISION_CONFLICT" ? "reload" : "none", stage: null, requestId: null, details: "Fixture debug details must not be displayed" };
}

export function createServerMock(options: { state?: "empty" | "servers"; failure?: () => "none" | "inUse" | "revision" | "unknown"; delay?: () => number } = {}) {
  let servers = options.state === "empty" ? [] : structuredClone(shellServers);
  let groups = options.state === "empty" ? [] : structuredClone(shellGroups);
  const commands: string[] = [];
  const keys = new Set<string>();
  const guard = async () => {
    const delay = options.delay?.() ?? 0;
    if (delay) await new Promise(resolve => setTimeout(resolve, delay));
    const failure = options.failure?.() ?? "none";
    if (failure === "inUse") throw fixtureError("SERVER_IN_USE", "errors.serverInUse");
    if (failure === "revision") throw fixtureError("REVISION_CONFLICT", "errors.revisionConflict");
    if (failure === "unknown") throw new Error("fixture database and secret debug details");
  };
  const find = (id: string) => {
    const server = servers.find(item => item.id === id);
    if (!server) throw fixtureError("RESOURCE_NOT_FOUND", "errors.serverNotFound");
    return server;
  };
  const revision = (actual: number, expected: number) => {
    if (actual !== expected) throw fixtureError("REVISION_CONFLICT", "errors.revisionConflict");
  };
  const transport = createMockIpc({
    app_get_info: () => shellAppInfo,
    group_list: () => structuredClone(groups),
    server_list: () => ({ items: structuredClone(servers), nextCursor: null }),
    server_appearance_list: () => [],
    server_get: ({ id }) => structuredClone(find(id)),
    server_runtime_stats_get: ({ serverId }) => ({ serverId, lastSuccessAtMs: null, lastFailureAtMs: null, lastPreflightAtMs: null, lastPreflightLatencyMs: null, lastFailureCode: null, updatedAtMs: 0 }),
    host_key_get: () => null,
    network_inspect: ({ host, detailed }) => networkFixture(host, detailed),
    connection_preflight: ({ host }) => preflightFixture(host),
    local_file_select: () => {
      const token = crypto.randomUUID();
      keys.add(token);
      return { token, displayName: "fixture_ed25519", purpose: "privateKey", expiresAtMs: Date.now() + 600000 };
    },
    server_create: async ({ profile, credential }) => {
      await guard();
      if (profile.authType === "privateKey" && !keys.has(profile.privateKeyToken ?? "")) throw fixtureError("RESOURCE_CLOSED", "errors.localFileTokenExpired");
      const server: ServerProfile = {
        ...profile, id: crypto.randomUUID(), name: profile.name || `${profile.username}@${profile.host}`,
        hasPrivateKey: profile.authType === "privateKey", hasSavedCredential: credential.mode === "replace",
        revision: 1, createdAtMs: Date.now(), updatedAtMs: Date.now(),
      };
      // Fixtures never retain the credential or the private key token in profile state.
      delete (server as Partial<typeof profile>).privateKeyToken;
      keys.delete(profile.privateKeyToken ?? "");
      servers.push(server);
      return { server: structuredClone(server), credentialCleanupPending: false };
    },
    server_update: async ({ serverId, expectedRevision, profile, credential }) => {
      await guard();
      const current = find(serverId);
      revision(current.revision, expectedRevision);
      const server: ServerProfile = { ...current, ...profile, name: profile.name || `${profile.username}@${profile.host}`, hasPrivateKey: profile.authType === "privateKey", hasSavedCredential: credential.mode === "keep" ? current.hasSavedCredential : credential.mode === "replace", revision: current.revision + 1, updatedAtMs: Date.now() };
      delete (server as Partial<typeof profile>).privateKeyToken;
      keys.delete(profile.privateKeyToken ?? "");
      servers = servers.map(item => item.id === server.id ? server : item);
      return { server: structuredClone(server), credentialCleanupPending: false };
    },
    server_delete: async ({ serverId, expectedRevision, removeCredentials }) => {
      await guard();
      revision(find(serverId).revision, expectedRevision);
      if (!removeCredentials) throw new Error("Fixture requires explicit credential disposition");
      servers = servers.filter(item => item.id !== serverId);
      return { credentialCleanupPending: false };
    },
    group_create: async ({ name, sortOrder }) => {
      await guard();
      const group = { id: crypto.randomUUID(), name, sortOrder, revision: 1, createdAtMs: Date.now(), updatedAtMs: Date.now() };
      groups.push(group);
      return structuredClone(group);
    },
    group_update: async ({ groupId, update }) => {
      await guard();
      const current = groups.find(item => item.id === groupId);
      if (!current) throw fixtureError("RESOURCE_NOT_FOUND", "errors.serverNotFound");
      revision(current.revision, update.expectedRevision);
      const group = { ...current, name: update.name, sortOrder: update.sortOrder, revision: current.revision + 1 };
      groups = groups.map(item => item.id === groupId ? group : item);
      return structuredClone(group);
    },
    group_delete: async ({ id, expectedRevision }) => {
      await guard();
      const current = groups.find(item => item.id === id);
      if (!current) throw fixtureError("RESOURCE_NOT_FOUND", "errors.serverNotFound");
      revision(current.revision, expectedRevision);
      groups = groups.filter(item => item.id !== id);
      servers = servers.map(item => item.groupId === id ? { ...item, groupId: null } : item);
    },
  });
  return {
    commands,
    async invoke<T>(command: string, args?: Record<string, unknown>) {
      commands.push(command);
      return transport.invoke<T>(command, args);
    },
  };
}
