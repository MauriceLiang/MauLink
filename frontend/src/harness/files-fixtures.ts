import type { Channel } from "@tauri-apps/api/core";
import type { RemoteFileEntry } from "../../../contracts/v1/RemoteFileEntry";
import type { SftpTransferSnapshot } from "../../../contracts/v1/SftpTransferSnapshot";
import type { ConnectionSnapshot } from "../../../contracts/v1/ConnectionSnapshot";
import type { TerminalSnapshot } from "../../../contracts/v1/TerminalSnapshot";
import type { IpcTransport } from "../ipc/client";
import { createMockIpc } from "../ipc/mock";
import { createServerMock, fixtureError } from "./server-fixtures";
import { createMonitorMock } from "./monitor-fixtures";
import { defaultSettings } from "../terminal/preferences";
import { joinRemotePath, validBasename } from "../files/path";
export type FileScenario = 'completed' | 'slow' | 'failed' | 'picker-cancel';
export function createFilesMock(scenario: () => FileScenario = () => 'completed') {
  const directories = new Map<string, RemoteFileEntry[]>();
  const textContents = new Map<string, string>();
  const cursors = new Map<string, { path: string; entries: RemoteFileEntry[]; offset: number }>();
  const tasks = new Map<string, SftpTransferSnapshot>(); const channels = new Map<string, Channel<SftpTransferSnapshot>>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>(); const calls: string[] = [];
  const entry = (parent: string, name: string, fileType: RemoteFileEntry['fileType'] = 'file'): RemoteFileEntry => ({ name, path: joinRemotePath(parent, name), fileType, sizeBytes: fileType === 'directory' ? null : '4096', modifiedAtMs: 1790812800000, isSymlink: fileType === 'symlink', permissions: 420 });
  function reset() {
    directories.clear(); cursors.clear(); textContents.clear();
    directories.set('/', [entry('/', 'fixture', 'directory')]);
    directories.set('/fixture', [entry('/fixture', 'empty', 'directory'), entry('/fixture', 'denied', 'directory'), entry('/fixture', 'nested', 'directory'), entry('/fixture', '目录链接', 'symlink'), entry('/fixture', '中文 "引号" \\ 文件.txt'), ...Array.from({ length: 445 }, (_, index) => entry('/fixture', `file-${String(index + 1).padStart(3, '0')}.txt`))]);
    directories.set('/fixture/empty', []); directories.set('/fixture/nested', [entry('/fixture/nested', 'readme.txt')]);
    textContents.set('/fixture/中文 "引号" \\ 文件.txt', 'MauLink 远程文本文件\n双击可查看，切换编辑后可保存。\n');
    textContents.set('/fixture/nested/readme.txt', '# Remote fixture\n\nThis is a plain text preview.');
  }
  reset();
  function directory(path: string) {
    if (path === '/fixture/denied') throw fixtureError('PERMISSION_DENIED', 'errors.permissionDenied');
    if (path === '/fixture/目录链接') path = '/fixture/nested';
    const values = directories.get(path); if (!values) throw fixtureError('PATH_NOT_FOUND', 'errors.pathNotFound');
    return { path, values };
  }
  function find(path: string) {
    const value = [...directories.values()].flat().find(item => item.path === path);
    if (!value) throw fixtureError('PATH_NOT_FOUND', 'errors.pathNotFound'); return value;
  }
  function textRevision(content: string) { return `fixture-${content.length}-${content}`; }
  function page(id: string) {
    const value = cursors.get(id); if (!value) throw fixtureError('RESOURCE_CLOSED', 'errors.sftpCursorClosed');
    const entries = value.entries.slice(value.offset, value.offset + 200); value.offset += entries.length;
    const more = value.offset < value.entries.length; if (!more) cursors.delete(id);
    return { path: value.path, entries: structuredClone(entries), cursorId: more ? id : null };
  }
  let output: Channel<SftpTransferSnapshot>;
  function publish(value: SftpTransferSnapshot) { tasks.set(value.transferId, value); channels.get(value.transferId)?.onmessage(structuredClone(value)); }
  function transfer(connectionId: string, remotePath: string, direction: 'upload' | 'download') {
    if (scenario() === 'slow' && [...tasks.values()].some(value => !['completed', 'failed', 'cancelled'].includes(value.state))) throw fixtureError('TRANSFER_BUSY', 'errors.transferBusy');
    if (direction === 'upload' && [...directories.values()].flat().some(item => item.path === remotePath)) throw fixtureError('TARGET_EXISTS', 'errors.targetExists');
    const id = crypto.randomUUID(); const mode = scenario();
    const initial: SftpTransferSnapshot = { transferId: id, connectionId, direction, fileName: remotePath.split('/').at(-1)!, finalPath: remotePath, totalBytes: '1048576', transferredBytes: '0', bytesPerSecond: null, remainingSeconds: null, state: 'created', error: null, cleanupRequired: false, temporaryPath: null };
    tasks.set(id, initial); channels.set(id, output);
    let step = 0; const tick = () => {
      const current = tasks.get(id)!; if (['cancelled', 'completed', 'failed'].includes(current.state)) return;
      step++; const finish = step >= (mode === 'slow' ? 80 : 4);
      const failed = finish && mode === 'failed';
      const value: SftpTransferSnapshot = { ...current, state: finish ? failed ? 'failed' : 'completed' : 'transferring', transferredBytes: finish && !failed ? '1048576' : String(Math.min(1048576, step * 65536)), bytesPerSecond: 262144, remainingSeconds: null, error: failed ? fixtureError('LOCAL_DISK_FULL', 'errors.localDiskFull') : null, cleanupRequired: failed, temporaryPath: failed ? '/fixture/.maulink-demo.part' : null };
      if (finish && !failed && direction === 'upload') { const parent = remotePath.slice(0, remotePath.lastIndexOf('/')) || '/'; directories.get(parent)?.push(entry(parent, value.fileName)); }
      publish(value); if (!finish) timers.set(id, setTimeout(tick, 300));
    };
    timers.set(id, setTimeout(tick, 300)); return structuredClone(initial);
  }
  const mock = createMockIpc({
    sftp_list_start: ({ path }) => { const found = directory(path === '.' || path === '' ? '/fixture' : path); const id = crypto.randomUUID(); cursors.set(id, { path: found.path, entries: structuredClone(found.values), offset: 0 }); return page(id); },
    sftp_list_next: ({ cursorId }) => page(cursorId), sftp_list_close: ({ cursorId }) => { cursors.delete(cursorId); },
    sftp_stat: ({ path, followSymlink }) => { const value = find(path); return followSymlink && value.isSymlink ? { ...value, fileType: 'directory', isSymlink: false } : structuredClone(value); },
    sftp_read_text: ({ path }) => { const value = find(path); if (value.fileType !== 'file' || value.isSymlink) throw fixtureError('VALIDATION_FAILED', 'errors.sftpTextFileUnsupported'); const content = textContents.get(path) ?? `Fixture text for ${value.name}\n`; textContents.set(path, content); return { content, revision: textRevision(content) }; },
    sftp_write_text: ({ path, content, expectedRevision }) => { const value = find(path); if (value.fileType !== 'file' || value.isSymlink) throw fixtureError('VALIDATION_FAILED', 'errors.sftpTextFileUnsupported'); const previous = textContents.get(path) ?? `Fixture text for ${value.name}\n`; if (textRevision(previous) !== expectedRevision) throw fixtureError('REVISION_CONFLICT', 'errors.sftpFileChangedDuringEdit'); textContents.set(path, content); return { revision: textRevision(content) }; },
    sftp_mkdir: ({ parentPath, name }) => { if (!validBasename(name)) throw fixtureError('VALIDATION_FAILED', 'errors.sftpNameInvalid'); const { values } = directory(parentPath); if (values.some(value => value.name === name)) throw fixtureError('TARGET_EXISTS', 'errors.targetExists'); const value = entry(parentPath, name, 'directory'); values.unshift(value); directories.set(value.path, []); return value; },
    sftp_rename: ({ sourcePath, newName }) => { const value = find(sourcePath); if (!validBasename(newName)) throw fixtureError('VALIDATION_FAILED', 'errors.sftpNameInvalid'); const parent = sourcePath.slice(0, sourcePath.lastIndexOf('/')) || '/'; if (directory(parent).values.some(item => item.name === newName)) throw fixtureError('TARGET_EXISTS', 'errors.targetExists'); if (value.fileType === 'directory') throw fixtureError('PERMISSION_DENIED', 'errors.permissionDenied'); value.name = newName; value.path = joinRemotePath(parent, newName); return structuredClone(value); },
    sftp_delete: ({ path, expectedType, confirmed }) => { const value = find(path); if (!confirmed || value.fileType !== expectedType) throw fixtureError('VALIDATION_FAILED', 'errors.sftpEntryTypeChanged'); if (directories.get(path)?.length) throw fixtureError('DIRECTORY_NOT_EMPTY', 'errors.directoryNotEmpty'); const parent = path.slice(0, path.lastIndexOf('/')) || '/'; directories.set(parent, directory(parent).values.filter(item => item.path !== path)); directories.delete(path); },
    local_file_select: ({ purpose }) => scenario() === 'picker-cancel' ? null : { token: crypto.randomUUID(), displayName: 'upload-demo.bin', purpose, expiresAtMs: Date.now() + 120000 },
    sftp_upload: ({ connectionId, remotePath }) => transfer(connectionId, remotePath, 'upload'), sftp_download: ({ connectionId, remotePath }) => transfer(connectionId, remotePath, 'download'),
    sftp_transfer_get: ({ transferId }) => structuredClone(tasks.get(transferId)!), sftp_transfer_list: ({ connectionId }) => [...tasks.values()].filter(value => !connectionId || value.connectionId === connectionId).reverse().map(value => structuredClone(value)),
    sftp_transfer_cancel: ({ transferId }) => { const value = tasks.get(transferId)!; clearTimeout(timers.get(transferId)); timers.set(transferId, setTimeout(() => publish({ ...value, state: 'cancelled', temporaryPath: null }), 600)); return structuredClone(value); },
  });
  const transport: IpcTransport = { async invoke<T>(command: string, args?: Record<string, unknown>) { calls.push(command); if (command === 'sftp_upload' || command === 'sftp_download') output = args?.outputChannel as Channel<SftpTransferSnapshot>; return mock.invoke<T>(command, args); } };
  function dispose() { timers.forEach(clearTimeout); timers.clear(); channels.clear(); }
  return { transport, calls, reset, dispose, expire: () => cursors.clear(), tasks };
}

export function createFilesWorkspaceMock(files: ReturnType<typeof createFilesMock>) {
  const servers = createServerMock(); const monitor = createMonitorMock();
  let connection: ConnectionSnapshot = { connectionId: 'files-fixture', serverId: null, mode: 'workspace', state: 'ready', hostKeyChallenge: null, authenticationChallenge: null, negotiatedAlgorithms: null, error: null, createdAtMs: 1, updatedAtMs: 1 };
  const terminals = new Map<string, TerminalSnapshot>();
  const shell = createMockIpc({
    connection_start: ({ source }) => { connection = { ...connection, serverId: source.kind === 'saved' ? source.serverId : null, state: 'ready' }; return connection; }, connection_get: () => connection,
    connection_disconnect: ({ stopActiveTransfers }) => {
      const active = [...files.tasks.values()].filter(value => !['completed', 'failed', 'cancelled'].includes(value.state));
      if (active.length && !stopActiveTransfers) throw { ...fixtureError('TRANSFER_BUSY', 'errors.activeTransfersRequireConfirmation'), params: { count: String(active.length) } };
      for (const value of active) files.tasks.set(value.transferId, { ...value, state: 'cancelled' });
      connection = { ...connection, state: 'closed' }; terminals.forEach(value => { value.state = 'closed'; }); return connection;
    },
    settings_get: () => ({ value: defaultSettings, revision: 1, updatedAtMs: 1 }),
    terminal_open: ({ connectionId, columns, rows }) => { const terminalId = crypto.randomUUID(); const streamId = crypto.randomUUID(); terminals.set(terminalId, { terminalId, streamId, connectionId, size: { columns, rows, pixelWidth: null, pixelHeight: null }, state: 'running', error: null, exitStatus: null, exitSignal: null, createdAtMs: 1, updatedAtMs: 1 }); return { terminalId, streamId }; },
    terminal_get: ({ terminalId }) => terminals.get(terminalId)!, terminal_resize: value => value, terminal_ack: () => undefined, terminal_write: ({ inputSeq }) => ({ inputSeq, duplicate: false }), terminal_close: ({ terminalId }) => { const value = terminals.get(terminalId)!; value.state = 'closed'; return value; },
  });
  const transport: IpcTransport = { async invoke<T>(command: string, args?: Record<string, unknown>) {
    if (command.startsWith('monitor_') || command === 'workspace_set_activity') return monitor.invoke<T>(command, args);
    if (command.startsWith('sftp_') || command === 'local_file_select') return files.transport.invoke<T>(command, args);
    if (command.startsWith('connection_') || command.startsWith('terminal_') || command === 'settings_get') return shell.invoke<T>(command, args);
    return servers.invoke<T>(command, args);
  } };
  return transport;
}
