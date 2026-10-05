import type { AppInfo } from "../ipc/commands";
import type { Group } from "../../../contracts/v1/Group";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import { createMockIpc } from "../ipc/mock";
import { networkFixture } from "./network-fixtures";

export const shellAppInfo: AppInfo = {
  name: "MauLink", version: "0.1.0", apiVersion: 1, platform: "macos", architecture: "aarch64",
  capabilities: { profileStorage: true, secureCredentials: true, ssh: true, terminal: true, sftp: true, monitor: true },
};
export const shellGroups: Group[] = [
  { id: "production", name: "Production", sortOrder: 0, revision: 1, createdAtMs: 0, updatedAtMs: 0 },
  { id: "development", name: "Development", sortOrder: 1, revision: 1, createdAtMs: 0, updatedAtMs: 0 },
];
const defaults = {
  port: 22, username: "root", authType: "password", hasPrivateKey: false, hasSavedCredential: false,
  connectTimeoutMs: 10000, keepaliveIntervalSeconds: 30, jumpHost: null, jumpPort: 22,
  proxyType: null, proxyHost: null, proxyPort: null, revision: 1, createdAtMs: 0, updatedAtMs: 0,
} satisfies Omit<ServerProfile, "id" | "name" | "host" | "groupId">;
export const shellServers: ServerProfile[] = [
  { ...defaults, id: "web-01", name: "Web-01", host: "192.168.1.20", groupId: "production" },
  { ...defaults, id: "db-01", name: "DB-01", host: "192.168.1.21", groupId: "production" },
  { ...defaults, id: "dev-01", name: "Dev-01", host: "dev.example.com", groupId: "development" },
];

export function createShellMock(state: "empty" | "servers" = "empty") {
  return createMockIpc({
    app_get_info: () => shellAppInfo,
    group_list: () => state === "servers" ? shellGroups : [],
    server_list: () => ({ items: state === "servers" ? shellServers : [], nextCursor: null }),
    host_key_get: () => null,
    network_inspect: ({ host, detailed }) => networkFixture(host, detailed),
  });
}
