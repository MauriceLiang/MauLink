import { computed, ref, shallowRef } from "vue";
import type { ConnectionSnapshot } from "../../../contracts/v1/ConnectionSnapshot";
import type { HostKeyDecision } from "../../../contracts/v1/HostKeyDecision";
import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerProfileDraft } from "../../../contracts/v1/ServerProfileDraft";
import type { createConnectionApi } from "../ipc/connection";
import { mapError } from "../errors/mapper";

export const isFinished = (snapshot: ConnectionSnapshot) => ["closed", "failed", "cancelled"].includes(snapshot.state);

export function createConnectionStore(api: ReturnType<typeof createConnectionApi>) {
  const snapshots = shallowRef<Record<string, ConnectionSnapshot>>({});
  const errors = shallowRef<Record<string, AppError | null>>({});
  const busy = ref<Record<string, boolean>>({});
  const answered = ref<string[]>([]);
  const draftIdentities = shallowRef<Record<string, { username: string; host: string }>>({});
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const versions = new Map<string, number>();
  const polling = new Set<string>();
  const pollErrors = new Set<string>();
  let disposed = false;
  const challenge = computed(() => Object.entries(snapshots.value).find(([, snapshot]) => {
    const id = snapshot.hostKeyChallenge?.challengeId ?? snapshot.authenticationChallenge?.challengeId;
    return id && !answered.value.includes(id) && !isFinished(snapshot);
  }));
  const draftTestActive = computed(() => Object.keys(draftIdentities.value).some(id => {
    const snapshot = snapshots.value[id];
    return !!snapshot && !isFinished(snapshot);
  }));
  function setError(id: string, error: AppError | null) { errors.value = { ...errors.value, [id]: error }; }
  function stop(id: string) { clearTimeout(timers.get(id)); timers.delete(id); }
  function accept(id: string, snapshot: ConnectionSnapshot) {
    if (disposed) return;
    snapshots.value = { ...snapshots.value, [id]: snapshot };
    stop(id);
    if (!isFinished(snapshot)) timers.set(id, setTimeout(() => { void refresh(id); }, snapshot.state === "ready" ? 1400 : 300));
  }
  async function refresh(id: string) {
    stop(id);
    const current = snapshots.value[id];
    if (disposed || !current || busy.value[id] || polling.has(id)) return;
    polling.add(id);
    const version = versions.get(id);
    try {
      const next = await api.get({ connectionId: current.connectionId });
      // An in-flight poll must not overwrite a later cancel or challenge response.
      if (!disposed && versions.get(id) === version) {
        if (pollErrors.delete(id)) setError(id, null);
        accept(id, next);
      }
    } catch (error) {
      if (!disposed && versions.get(id) === version) { pollErrors.add(id); setError(id, mapError(error)); }
      // Retain the Core snapshot on transport failure; let the user refresh or cancel.
    } finally { polling.delete(id); }
  }
  async function run(id: string, action: () => Promise<void>) {
    if (disposed || busy.value[id]) return;
    busy.value[id] = true;
    versions.set(id, (versions.get(id) ?? 0) + 1);
    stop(id);
    pollErrors.delete(id);
    setError(id, null);
    try { await action(); }
    catch (error) { if (!disposed) setError(id, mapError(error)); }
    finally {
      busy.value[id] = false;
      const snapshot = snapshots.value[id];
      if (!disposed && snapshot && !isFinished(snapshot)) timers.set(id, setTimeout(() => { void refresh(id); }, 300));
    }
  }
  async function start(server: ServerProfile) {
    const current = snapshots.value[server.id];
    if (challenge.value || (current && !isFinished(current))) return;
    await run(server.id, async () => {
      const next = await api.start({ source: { kind: "saved", serverId: server.id, expectedRevision: server.revision }, mode: "workspace" });
      accept(server.id, next);
    });
  }
  async function startDraftTest(profile: ServerProfileDraft, credential: string | null, savedProfile?: { serverId: string; expectedRevision: number; useSavedCredential: boolean }) {
    if (draftTestActive.value || challenge.value) return null;
    const source = savedProfile
      ? { kind: "draftWithSavedProfile" as const, profile, ...savedProfile, credential }
      : { kind: "draft" as const, profile, credential };
    const snapshot = await api.start({ source, mode: "test" });
    draftIdentities.value = { ...draftIdentities.value, [snapshot.connectionId]: { username: profile.username, host: profile.host } };
    accept(snapshot.connectionId, snapshot);
    return snapshot;
  }
  function identityForChallenge(id: string) { return draftIdentities.value[id] ?? null; }
  async function cancel(id: string) {
    const snapshot = snapshots.value[id];
    if (!snapshot || isFinished(snapshot) || snapshot.state === "ready") return;
    await run(id, async () => { accept(id, await api.cancel({ connectionId: snapshot.connectionId })); });
  }
  async function disconnect(id: string, stopActiveTransfers = false) {
    const snapshot = snapshots.value[id];
    if (!snapshot || snapshot.state !== "ready") return;
    await run(id, async () => { accept(id, await api.disconnect({ connectionId: snapshot.connectionId, stopActiveTransfers })); });
  }
  async function respondHostKey(id: string, decision: HostKeyDecision) {
    const snapshot = snapshots.value[id];
    const key = snapshot?.hostKeyChallenge;
    if (!key || snapshot.state !== "awaitingHostTrust" || answered.value.includes(key.challengeId)) return;
    await run(id, async () => {
      await api.respondHostKey({ connectionId: snapshot.connectionId, challengeId: key.challengeId, decision });
      answered.value = [...answered.value, key.challengeId];
    });
  }
  async function respondAuthentication(id: string, secret: string) {
    const snapshot = snapshots.value[id];
    const auth = snapshot?.authenticationChallenge;
    if (!auth || snapshot.state !== "awaitingCredentials" || answered.value.includes(auth.challengeId)) return;
    await run(id, async () => {
      await api.respondAuthentication({ connectionId: snapshot.connectionId, challengeId: auth.challengeId, secret });
      answered.value = [...answered.value, auth.challengeId];
    });
  }
  function dismiss(id: string) {
    const snapshot = snapshots.value[id];
    if (busy.value[id] || (snapshot && !isFinished(snapshot))) return;
    stop(id);
    const next = { ...snapshots.value }; delete next[id]; snapshots.value = next;
    const identities = { ...draftIdentities.value }; delete identities[id]; draftIdentities.value = identities;
    setError(id, null);
  }
  function dispose() { disposed = true; timers.forEach(clearTimeout); timers.clear(); }
  return { snapshots, errors, busy, challenge, draftTestActive, start, startDraftTest, identityForChallenge, cancel, disconnect, respondHostKey, respondAuthentication, refresh, dismiss, dispose };
}
export type ConnectionStore = ReturnType<typeof createConnectionStore>;
