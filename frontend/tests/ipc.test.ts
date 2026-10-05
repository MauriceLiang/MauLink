import { describe, expect, it, vi } from "vitest";
import { createIpcClient, createRequest } from "../src/ipc/client";
import { createMockIpc } from "../src/ipc/mock";
import { createServerApi } from "../src/ipc/server";
import { createConnectionApi } from "../src/ipc/connection";
import { createTerminalApi } from "../src/ipc/terminal";
import { createSftpApi } from "../src/ipc/sftp";
import { createMonitorApi } from "../src/ipc/monitor";
import { createSettingsApi } from "../src/ipc/settings";
import { createHostKeysApi } from "../src/ipc/host-keys";
import { createNetworkApi } from "../src/ipc/network";
import { createPreflightApi } from "../src/ipc/preflight";
import { emptyServers, timeoutError } from "../src/harness/fixtures";

describe("typed IPC contract", () => {
  it("uses API v1 and a fresh UUID on every request", () => {
    const requests = Array.from({ length: 32 }, () => createRequest({ connectionId: "fixture" }));
    expect(new Set(requests.map(request => request.requestId)).size).toBe(32);
    for (const request of requests) {
      expect(request.apiVersion).toBe(1);
      expect(request.requestId).toMatch(/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/);
      expect(request.payload).toEqual({ connectionId: "fixture" });
    }
  });

  it("routes domain facades through their real command names and envelopes", async () => {
    const transport = createMockIpc({});
    const invoke = vi.spyOn(transport, "invoke").mockResolvedValue(undefined);
    const client = createIpcClient(transport);
    const query = { query: null, groupId: null, limit: 20, cursor: null };
    await createServerApi(client).list(query);
    await createConnectionApi(client).get({ connectionId: "connection" });
    await createTerminalApi(client).ack({ terminalId: "terminal", streamId: "stream", seq: "1" });
    await createSftpApi(client).listStart({ connectionId: "connection", path: "/" });
    await createMonitorApi(client).getSnapshot({ connectionId: "connection" });
    await createSettingsApi(client).get();
    await createHostKeysApi(client).get({ host: "example.com", port: 22 });
    await createNetworkApi(client).inspect({ host: "example.com", detailed: true });
    await createPreflightApi(client).check({ host: "example.com", port: 22, timeoutMs: 10000 });
    expect(invoke.mock.calls.map(([name]) => name)).toEqual([
      "server_list", "connection_get", "terminal_ack", "sftp_list_start", "monitor_get_snapshot", "settings_get", "host_key_get", "network_inspect", "connection_preflight",
    ]);
    expect(invoke.mock.calls[0]?.[1]).toEqual({ request: { apiVersion: 1, requestId: expect.any(String), payload: query } });
  });

  it("keeps app_get_info as the existing envelope-free exception", async () => {
    const transport = createMockIpc({});
    const invoke = vi.spyOn(transport, "invoke").mockResolvedValue(undefined);
    await createIpcClient(transport).getInfo();
    expect(invoke).toHaveBeenCalledWith("app_get_info");
  });

  it("preserves structured backend failures with the request correlation id", async () => {
    const client = createIpcClient(createMockIpc({ server_list: () => { throw timeoutError; } }));
    await expect(createServerApi(client).list({ query: null, groupId: null, limit: 20, cursor: null }))
      .rejects.toMatchObject({ ...timeoutError, requestId: expect.any(String) });
  });

  it("runs typed fixtures without Tauri and rejects unconfigured mock commands", async () => {
    const client = createIpcClient(createMockIpc({ server_list: () => emptyServers }));
    await expect(createServerApi(client).list({ query: null, groupId: null, limit: 20, cursor: null })).resolves.toEqual(emptyServers);
    await expect(createSettingsApi(client).get()).rejects.toMatchObject({ code: "INTERNAL", messageKey: "errors.unexpected" });
  });
});
