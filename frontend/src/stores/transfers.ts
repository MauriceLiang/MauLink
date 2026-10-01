import { ref, shallowRef } from "vue";
import { Channel } from "@tauri-apps/api/core";
import type { SftpTransferSnapshot } from "../../../contracts/v1/SftpTransferSnapshot";
import type { AppError } from "../../../contracts/v1/AppError";
import type { SelectedLocalFile } from "../../../contracts/v1/SelectedLocalFile";
import type { createSftpApi } from "../ipc/sftp";
import { mapError } from "../errors/mapper";
import { joinRemotePath } from "../files/path";
export type TransferChannelFactory = () => Channel<SftpTransferSnapshot>;
export const transferFinished = (value: SftpTransferSnapshot) => ['completed', 'failed', 'cancelled'].includes(value.state);
const rank = { created: 0, transferring: 1, finalizing: 2, completed: 3, failed: 3, cancelled: 3 };
const lessBytes = (a: string, b: string) => a.length === b.length ? a < b : a.length < b.length;

export function createTransferStore(api: ReturnType<typeof createSftpApi>, select: (purpose: 'upload' | 'download') => Promise<SelectedLocalFile | null>, channelFactory: TransferChannelFactory = () => new Channel<SftpTransferSnapshot>()) {
  const snapshots = shallowRef<SftpTransferSnapshot[]>([]);
  const errors = shallowRef<Record<string, AppError | null>>({});
  const starting = ref<Record<string, boolean>>({}); const cancelling = ref<string[]>([]);
  const channels = new Set<Channel<SftpTransferSnapshot>>();
  const polls = new Set<string>(); const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const dismissed = new Set<string>();
  let disposed = false;
  function setError(id: string, value: unknown) { errors.value = { ...errors.value, [id]: value === null ? null : mapError(value) }; }
  function remember(value: SftpTransferSnapshot) {
    if (disposed || dismissed.has(value.transferId)) return;
    const old = snapshots.value.find(item => item.transferId === value.transferId);
    if (old && transferFinished(old)) cancelling.value = cancelling.value.filter(id => id !== value.transferId);
    // A late start/poll/cancel response cannot overwrite newer Channel progress or a terminal state.
    if (old && (transferFinished(old) || rank[value.state] < rank[old.state] || (!transferFinished(value) && lessBytes(value.transferredBytes, old.transferredBytes)))) return;
    const values = [value, ...snapshots.value.filter(item => item.transferId !== value.transferId)];
    // Trim terminal history first, as Core does, so another connection's history cannot hide active tasks.
    while (values.length > 50) { let index = values.length - 1; while (index >= 0 && !transferFinished(values[index]!)) index--; if (index < 0) break; values.splice(index, 1); }
    snapshots.value = values;
    clearTimeout(timers.get(value.transferId)); timers.delete(value.transferId);
    if (transferFinished(value)) cancelling.value = cancelling.value.filter(id => id !== value.transferId);
    else timers.set(value.transferId, setTimeout(() => { void poll(value.transferId); }, 1200));
  }
  async function poll(id: string) {
    if (disposed || polls.has(id)) return;
    clearTimeout(timers.get(id)); timers.delete(id);
    polls.add(id);
    try { remember(await api.getTransfer({ transferId: id })); }
    catch (reason) { if (!disposed && !snapshots.value.some(value => value.transferId === id && transferFinished(value))) { setError(id, reason); timers.set(id, setTimeout(() => { void poll(id); }, 2000)); } }
    finally {
      polls.delete(id);
      const current = snapshots.value.find(value => value.transferId === id);
      if (!disposed && current && !transferFinished(current) && !timers.has(id)) timers.set(id, setTimeout(() => { void poll(id); }, 1200));
    }
  }
  async function load(connectionId: string) {
    try { const values = await api.listTransfers({ connectionId, limit: 50 }); values.slice().reverse().forEach(remember); }
    catch (reason) { if (!disposed) setError(connectionId, reason); }
  }
  async function start(direction: 'upload' | 'download', connectionId: string, remotePath: string, allowed: () => boolean) {
    if (disposed || starting.value[connectionId] || !allowed()) return false;
    starting.value[connectionId] = true; setError(connectionId, null);
    let channel: Channel<SftpTransferSnapshot> | undefined;
    try {
      if (direction === 'download') {
        const target = await api.stat({ connectionId, path: remotePath, followSymlink: true });
        if (target.fileType !== 'file') throw { ...mapError(null), code: 'VALIDATION_FAILED', messageKey: 'errors.sftpTransferFileTypeUnsupported' };
      }
      if (disposed || !allowed()) return false;
      const local = await select(direction);
      if (!local || disposed || !allowed()) return false;
      channel = channelFactory(); channels.add(channel);
      const stream = channel;
      stream.onmessage = value => {
        if (value.connectionId !== connectionId) return;
        remember(value);
        if (transferFinished(value)) { channels.delete(stream); stream.onmessage = () => undefined; }
      };
      const payload = { connectionId, localFileToken: local.token, remotePath: direction === 'upload' ? joinRemotePath(remotePath, local.displayName) : remotePath };
      remember(await (direction === 'upload' ? api.upload(payload, stream) : api.download(payload, stream)));
      return true;
    } catch (reason) { if (!disposed) setError(connectionId, reason); if (channel) { channel.onmessage = () => undefined; channels.delete(channel); } return false; }
    finally { starting.value[connectionId] = false; }
  }
  async function cancel(id: string) {
    if (disposed || cancelling.value.includes(id)) return;
    cancelling.value = [...cancelling.value, id]; setError(id, null);
    try { remember(await api.cancelTransfer({ transferId: id })); }
    catch (reason) { if (!disposed) { setError(id, reason); cancelling.value = cancelling.value.filter(value => value !== id); } }
    // Core cancellation is asynchronous. Keep the current state until Channel/poll confirms it.
  }
  function clearCompleted(connectionId: string) { snapshots.value.filter(value => value.connectionId === connectionId && value.state === 'completed').forEach(value => dismissed.add(value.transferId)); snapshots.value = snapshots.value.filter(value => value.connectionId !== connectionId || value.state !== 'completed'); }
  function dispose() { disposed = true; timers.forEach(clearTimeout); timers.clear(); channels.forEach(channel => { channel.onmessage = () => undefined; }); channels.clear(); }
  return { snapshots, errors, starting, cancelling, load, start, cancel, clearCompleted, dispose };
}
export type TransferStore = ReturnType<typeof createTransferStore>;
