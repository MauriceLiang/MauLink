import { ref, shallowRef } from "vue";
import type { RemoteFileEntry } from "../../../contracts/v1/RemoteFileEntry";
import type { AppError } from "../../../contracts/v1/AppError";
import type { createSftpApi } from "../ipc/sftp";
import { mapError } from "../errors/mapper";

export function createFilesStore(api: ReturnType<typeof createSftpApi>, connectionId: string) {
  const entries = shallowRef<RemoteFileEntry[]>([]);
  const path = ref(''); const cursor = ref<string | null>(null); const page = ref(0);
  const pending = ref(false); const mutating = ref(false); const needsReload = ref(false);
  const error = shallowRef<AppError | null>(null); const cleanupError = shallowRef<AppError | null>(null);
  let generation = 0; let disposed = false;
  async function release(id: string | null) {
    if (!id) return;
    try { await api.listClose({ cursorId: id }); }
    catch (reason) { const mapped = mapError(reason); if (mapped.code !== 'RESOURCE_CLOSED') cleanupError.value = mapped; }
  }
  async function load(destination = path.value, next = false) {
    if (disposed || mutating.value || (next && (pending.value || !cursor.value || needsReload.value))) return;
    const token = ++generation; const previous = cursor.value; cursor.value = null;
    pending.value = true; error.value = null;
    if (!next) { entries.value = []; page.value = 0; await release(previous); }
    if (disposed || token !== generation) return;
    try {
      const result = next ? await api.listNext({ cursorId: previous! }) : await api.listStart({ connectionId, path: destination });
      if (disposed || token !== generation) { await release(result.cursorId); return; }
      // One Core page at a time: at most 200 directory rows, never an unbounded append.
      entries.value = result.entries; path.value = result.path; cursor.value = result.cursorId;
      page.value = next ? page.value + 1 : 1; needsReload.value = false;
    } catch (reason) {
      if (!disposed && token === generation) { error.value = mapError(reason); needsReload.value = true; }
      await release(previous);
    } finally { if (token === generation) pending.value = false; }
  }
  async function mutate(action: () => Promise<unknown>) {
    if (disposed || pending.value || mutating.value || needsReload.value || !path.value) return false;
    mutating.value = true; error.value = null;
    try { await action(); return true; }
    catch (reason) { if (!disposed) error.value = mapError(reason); return false; }
    finally { mutating.value = false; }
  }
  async function open(entry: RemoteFileEntry) {
    if (disposed || pending.value || mutating.value || needsReload.value) return;
    const token = generation;
    if (entry.fileType === 'directory') return load(entry.path);
    if (entry.isSymlink || entry.fileType === 'symlink') {
      try { const target = await api.stat({ connectionId, path: entry.path, followSymlink: true }); if (!disposed && token === generation && target.fileType === 'directory') await load(entry.path); }
      catch (reason) { if (!disposed && token === generation) error.value = mapError(reason); }
    }
  }
  const mkdir = (name: string) => mutate(() => api.mkdir({ connectionId, parentPath: path.value, name }));
  const rename = (entry: RemoteFileEntry, newName: string) => mutate(() => api.rename({ connectionId, sourcePath: entry.path, newName }));
  const remove = (entry: RemoteFileEntry) => mutate(() => api.remove({ connectionId, path: entry.path, expectedType: entry.isSymlink ? 'symlink' : entry.fileType, confirmed: true }));
  function suspend() { ++generation; pending.value = false; needsReload.value = true; const old = cursor.value; cursor.value = null; void release(old); }
  function dispose() { disposed = true; suspend(); }
  return { entries, path, cursor, page, pending, mutating, needsReload, error, cleanupError, load, open, mkdir, rename, remove, suspend, dispose };
}
export type FilesStore = ReturnType<typeof createFilesStore>;
