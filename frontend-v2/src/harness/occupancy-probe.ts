import type { IpcClient } from "../ipc/client";
import { createServerApi } from "../ipc/server";
import { createConnectionApi } from "../ipc/connection";

// This DEV-only probe exercises the existing ServerInUse command boundary.
// It cannot select real remote profiles, credentials, proxies or jump hosts.
export async function startOccupancyProbe(client: IpcClient) {
  const api = createServerApi(client);
  const page = await api.list({ query: "Phase4-占用验收", groupId: null, cursor: null, limit: 200 });
  const candidates = page.items.filter(server => server.name === "Phase4-占用验收");
  if (candidates.length !== 1 || page.nextCursor !== null) return null;
  const server = await api.get({ id: candidates[0]!.id });
  if (server.name !== "Phase4-占用验收" || server.host !== "127.0.0.1" || server.port !== 42424
    || server.username !== "phase4" || server.authType !== "password" || server.hasSavedCredential
    || server.jumpHost !== null || server.proxyType !== null || server.connectTimeoutMs !== 120000) return null;
  return createConnectionApi(client).start({ source: { kind: "saved", serverId: server.id, expectedRevision: server.revision }, mode: "test" });
}
