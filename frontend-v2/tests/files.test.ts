import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import type { Channel } from '@tauri-apps/api/core';
import type { SftpTransferSnapshot } from '../../contracts/v1/SftpTransferSnapshot';
import type { RemoteFileEntry } from '../../contracts/v1/RemoteFileEntry';
import type { SelectedLocalFile } from '../../contracts/v1/SelectedLocalFile';
import type { SftpDirectoryPage } from '../../contracts/v1/SftpDirectoryPage';
import { createIpcClient } from '../src/ipc/client';
import { createMockIpc, type MockHandlers } from '../src/ipc/mock';
import { createSftpApi } from '../src/ipc/sftp';
import { createFilesStore } from '../src/stores/files';
import { createTransferStore } from '../src/stores/transfers';
import { breadcrumbs, joinRemotePath, validBasename, formatSize } from '../src/files/path';
import { createFilesMock } from '../src/harness/files-fixtures';
import { fixtureError } from '../src/harness/server-fixtures';
import FilesHarness from '../src/harness/FilesHarness.vue';
import FileMenu from '../src/components/files/FileMenu.vue';
import TransferPanel from '../src/components/files/TransferPanel.vue';
const disposals: (() => void)[] = []; const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(value => value.unmount()); disposals.splice(0).forEach(dispose => dispose()); vi.useRealTimers(); document.body.innerHTML = ''; });
const apiFor = (handlers: MockHandlers) => createSftpApi(createIpcClient(createMockIpc(handlers)));
const initial: SftpTransferSnapshot = { transferId: 'task-a', connectionId: 'conn-a', direction: 'upload', fileName: 'a.txt', finalPath: '/fixture/a.txt', totalBytes: '18446744073709551615', transferredBytes: '0', bytesPerSecond: null, remainingSeconds: null, state: 'created', error: null, cleanupRequired: false, temporaryPath: null };
function tasks(handlers: MockHandlers = {}, picker = vi.fn(async () => ({ token: 'opaque', displayName: 'a.txt', purpose: 'upload' as const, expiresAtMs: Date.now() + 120000 }))) {
  const channel = { onmessage: (_value: SftpTransferSnapshot) => undefined } as Channel<SftpTransferSnapshot>;
  const upload = vi.fn(() => initial); const cancel = vi.fn(() => ({ ...initial, state: 'transferring' as const }));
  const store = createTransferStore(apiFor({ sftp_upload: upload, sftp_transfer_get: () => initial, sftp_transfer_list: () => [], sftp_transfer_cancel: cancel, ...handlers }), picker, () => channel);
  disposals.push(store.dispose); return { store, channel, picker, upload, cancel };
}
async function harness() { const wrapper = mount(FilesHarness, { attachTo: document.body, global: { stubs: { Teleport: true } } }); wrappers.push(wrapper); await flushPromises(); return wrapper; }
const button = (wrapper: VueWrapper, text: string) => wrapper.findAll('button').find(value => value.text() === text)!;
describe('remote paths and directory lifecycle', () => {
  it('preserves POSIX basenames, quotes, Unicode and backslashes without shell escaping', () => {
    const name = '中文 "引号" \\ $file.txt'; expect(validBasename(name)).toBe(true); expect(joinRemotePath('/home/', name)).toBe(`/home/${name}`);
    for (const invalid of ['', '.', '..', 'a/b', 'a\0b']) expect(validBasename(invalid)).toBe(false);
    expect(breadcrumbs('/home/中文')).toEqual([{ name: '/', path: '/' }, { name: 'home', path: '/home' }, { name: '中文', path: '/home/中文' }]);
    expect(formatSize('18446744073709551615')).toContain('EB');
  });
  it('replaces each 200-row Core page and closes the cursor before navigating away', async () => {
    const mock = createFilesMock(); disposals.push(mock.dispose); const files = createFilesStore(createSftpApi(createIpcClient(mock.transport)), 'conn-a'); disposals.push(files.dispose);
    await files.load('.'); expect(files.entries.value).toHaveLength(200); expect(files.path.value).toBe('/fixture');
    const first = files.entries.value[0]!.path; await files.load(files.path.value, true); expect(files.page.value).toBe(2); expect(files.entries.value).toHaveLength(200); expect(files.entries.value[0]!.path).not.toBe(first);
    await files.load('/fixture/empty'); expect(files.entries.value).toHaveLength(0); expect(mock.calls).toContain('sftp_list_close');
    await files.load('.'); await files.load(files.path.value, true); await files.load(files.path.value, true); expect(files.entries.value).toHaveLength(50); expect(files.cursor.value).toBeNull();
  });
  it('releases a late abandoned directory response instead of overwriting the current path', async () => {
    let finish!: (page: SftpDirectoryPage) => void; const close = vi.fn();
    const files = createFilesStore(apiFor({ sftp_list_start: ({ path }) => path === '/old' ? new Promise(resolve => { finish = resolve; }) : { path, entries: [], cursorId: null }, sftp_list_close: close }), 'conn-a'); disposals.push(files.dispose);
    const old = files.load('/old'); await flushPromises(); await files.load('/current'); finish({ path: '/old', entries: [], cursorId: 'abandoned' }); await old;
    expect(files.path.value).toBe('/current'); expect(close).toHaveBeenCalledWith({ cursorId: 'abandoned' }); expect(files.pending.value).toBe(false);
  });
  it('does not navigate on a late symlink stat after disconnect', async () => {
    let finish!: (page: RemoteFileEntry) => void;
    const start = vi.fn(() => ({ path: '/fixture', entries: [], cursorId: null }));
    const files = createFilesStore(apiFor({ sftp_list_start: start, sftp_stat: () => new Promise(resolve => { finish = resolve; }) }), 'conn-a'); disposals.push(files.dispose); await files.load('.');
    const opening = files.open({ name: 'link', path: '/fixture/link', fileType: 'symlink', isSymlink: true, sizeBytes: null, modifiedAtMs: null, permissions: null }); files.suspend(); finish({ name: 'link', path: '/fixture/link', fileType: 'directory', isSymlink: false, sizeBytes: null, modifiedAtMs: null, permissions: null }); await opening;
    expect(start).toHaveBeenCalledTimes(1); expect(files.needsReload.value).toBe(true);
  });
  it('retains failed-page rows and blocks mutations until a fresh directory succeeds', async () => {
    const mock = createFilesMock(); disposals.push(mock.dispose); const files = createFilesStore(createSftpApi(createIpcClient(mock.transport)), 'conn-a'); disposals.push(files.dispose);
    await files.load('.'); mock.expire(); await files.load(files.path.value, true); expect(files.needsReload.value).toBe(true); expect(files.entries.value).toHaveLength(200);
    expect(await files.mkdir('unsafe')).toBe(false); expect(mock.calls).not.toContain('sftp_mkdir'); await files.load(); expect(files.needsReload.value).toBe(false);
  });
  it('keeps literal paths and deletion type/confirmation in typed IPC payloads', async () => {
    const remove = vi.fn(); const rename = vi.fn(value => ({ name: value.newName, path: value.sourcePath, fileType: 'file' as const, isSymlink: false, sizeBytes: null, modifiedAtMs: null, permissions: null }));
    const files = createFilesStore(apiFor({ sftp_list_start: () => ({ path: '/fixture', entries: [], cursorId: null }), sftp_delete: remove, sftp_rename: rename }), 'conn-a'); disposals.push(files.dispose); await files.load('.');
    const entry = { name: 'link', path: '/fixture/中文 "\\"', fileType: 'symlink' as const, isSymlink: true, sizeBytes: null, modifiedAtMs: null, permissions: null };
    await files.remove(entry); expect(remove).toHaveBeenCalledWith({ connectionId: 'conn-a', path: entry.path, expectedType: 'symlink', confirmed: true });
    await files.rename(entry, 'new\\name'); expect(rename).toHaveBeenCalledWith({ connectionId: 'conn-a', sourcePath: entry.path, newName: 'new\\name' });
  });
});
describe('transfer metadata and authoritative cancellation', () => {
  it('does not invoke transfer IPC when the picker is cancelled', async () => {
    const upload = vi.fn(); const store = createTransferStore(apiFor({ sftp_upload: upload }), async () => null); disposals.push(store.dispose);
    expect(await store.start('upload', 'conn-a', '/fixture', () => true)).toBe(false); expect(upload).not.toHaveBeenCalled(); expect(store.snapshots.value).toHaveLength(0);
  });
  it('rejects a directory download before opening the picker', async () => {
    const { store, picker } = tasks({ sftp_stat: () => ({ name: 'folder', path: '/folder', fileType: 'directory', sizeBytes: null, modifiedAtMs: null, isSymlink: false, permissions: null }) });
    expect(await store.start('download', 'conn-a', '/folder', () => true)).toBe(false); expect(picker).not.toHaveBeenCalled(); expect(store.errors.value['conn-a']?.messageKey).toBe('errors.sftpTransferFileTypeUnsupported');
  });
  it('abandons a selected token after its connection/path context changes', async () => {
    let finish!: (value: SelectedLocalFile) => void; let allowed = true; const upload = vi.fn();
    const store = createTransferStore(apiFor({ sftp_upload: upload }), () => new Promise(resolve => { finish = resolve; })); disposals.push(store.dispose);
    const pending = store.start('upload', 'conn-a', '/fixture', () => allowed); allowed = false; finish({ token: 'opaque', displayName: 'a.txt', purpose: 'upload', expiresAtMs: 1 }); await pending; expect(upload).not.toHaveBeenCalled();
  });
  it('does not downgrade early Channel completion with the later start response', async () => {
    let finish!: (value: SftpTransferSnapshot) => void;
    const { store, channel } = tasks({ sftp_upload: () => new Promise(resolve => { finish = resolve; }) });
    const starting = store.start('upload', 'conn-a', '/fixture', () => true); await flushPromises(); channel.onmessage({ ...initial, state: 'completed', transferredBytes: initial.totalBytes! }); finish(initial); await starting;
    expect(store.snapshots.value[0]?.state).toBe('completed'); expect(store.snapshots.value[0]?.transferredBytes).toBe('18446744073709551615'); expect(Object.keys(store.snapshots.value[0]!)).not.toContain('bytes');
  });
  it('keeps async cancel pending until Channel confirms a terminal state, including completion races', async () => {
    const { store, channel, cancel } = tasks(); await store.start('upload', 'conn-a', '/fixture', () => true); channel.onmessage({ ...initial, state: 'transferring', transferredBytes: '9007199254741000' });
    await store.cancel(initial.transferId); expect(cancel).toHaveBeenCalledWith({ transferId: initial.transferId }); expect(store.snapshots.value[0]?.state).toBe('transferring'); expect(store.cancelling.value).toContain(initial.transferId);
    channel.onmessage({ ...initial, state: 'completed', transferredBytes: initial.totalBytes! }); expect(store.snapshots.value[0]?.state).toBe('completed'); expect(store.cancelling.value).toHaveLength(0);
  });
  it('uses Core poll recovery when a progress Channel is silent', async () => {
    vi.useFakeTimers(); const get = vi.fn(() => ({ ...initial, state: 'cancelled' as const })); const { store } = tasks({ sftp_transfer_get: get });
    await store.start('upload', 'conn-a', '/fixture', () => true); await vi.advanceTimersByTimeAsync(1200); expect(get).toHaveBeenCalledWith({ transferId: initial.transferId }); expect(store.snapshots.value[0]?.state).toBe('cancelled');
  });
  it('continues recovery polling after a stale response is rejected', async () => {
    vi.useFakeTimers(); let count = 0; const get = vi.fn(() => ++count === 1 ? initial : { ...initial, state: 'completed' as const });
    const { store, channel } = tasks({ sftp_transfer_get: get }); await store.start('upload', 'conn-a', '/fixture', () => true); channel.onmessage({ ...initial, state: 'transferring', transferredBytes: '100' });
    await vi.advanceTimersByTimeAsync(1200); expect(store.snapshots.value[0]?.state).toBe('transferring'); await vi.advanceTimersByTimeAsync(1200); expect(store.snapshots.value[0]?.state).toBe('completed'); expect(get).toHaveBeenCalledTimes(2);
  });
  it('clears completed metadata without resurrecting it on history reload or cancelling tasks on dispose', async () => {
    const { store, channel, cancel } = tasks({ sftp_transfer_list: () => [{ ...initial, state: 'completed' }] }); await store.start('upload', 'conn-a', '/fixture', () => true); channel.onmessage({ ...initial, state: 'completed' }); store.clearCompleted('conn-a'); await store.load('conn-a');
    expect(store.snapshots.value).toHaveLength(0); store.dispose(); expect(cancel).not.toHaveBeenCalled();
  });
  it('retains active tasks when loading a full page of terminal history', async () => {
    const { store } = tasks({ sftp_transfer_list: () => Array.from({ length: 50 }, (_, index) => ({ ...initial, transferId: `history-${index}`, state: 'completed' })) });
    await store.start('upload', 'conn-a', '/fixture', () => true); await store.load('conn-a'); expect(store.snapshots.value).toHaveLength(50); expect(store.snapshots.value.some(value => value.transferId === initial.transferId && value.state === 'created')).toBe(true);
  });
  it('shows zero-byte completed files as complete rather than an indeterminate transfer', async () => {
    const { store } = tasks({ sftp_transfer_list: () => [{ ...initial, totalBytes: '0', transferredBytes: '0', state: 'completed' }] }); await store.load('conn-a');
    const wrapper = mount(TransferPanel, { props: { store, connectionId: 'conn-a' } }); wrappers.push(wrapper); expect(wrapper.get('progress').attributes('value')).toBe('100'); expect(wrapper.text()).toContain('已完成');
  });
  it('presents Core failure and cleanup warnings without raw diagnostic details', async () => {
    const { store, channel } = tasks(); await store.start('upload', 'conn-a', '/fixture', () => true); channel.onmessage({ ...initial, state: 'failed', error: fixtureError('LOCAL_DISK_FULL', 'errors.localDiskFull'), cleanupRequired: true, temporaryPath: '/fixture/.part' });
    const wrapper = mount(TransferPanel, { props: { store, connectionId: 'conn-a' } }); wrappers.push(wrapper); expect(wrapper.text()).toContain('磁盘空间不足'); expect(wrapper.text()).toContain('/fixture/.part'); expect(wrapper.text()).not.toContain('Fixture debug'); expect(wrapper.findAll('button').some(value => value.text() === '取消传输')).toBe(false);
  });
});
describe('file UI acceptance', () => {
  it('renders bounded pages, empty/denied locations, and keeps preview/edit disabled', async () => {
    const wrapper = await harness(); expect(wrapper.findAll('tbody tr')).toHaveLength(200); expect(button(wrapper, '查看 / 编辑').attributes('disabled')).toBeDefined();
    await button(wrapper, '下一页').trigger('click'); await flushPromises(); expect(wrapper.findAll('tbody tr')).toHaveLength(200); expect(wrapper.text()).toContain('第 2 页');
    await wrapper.get('[aria-label="远程路径"]').setValue('/fixture/empty'); await wrapper.get('.files-location').trigger('submit'); await flushPromises(); expect(wrapper.text()).toContain('目录为空');
    await wrapper.get('[aria-label="远程路径"]').setValue('/fixture/denied'); await wrapper.get('.files-location').trigger('submit'); await flushPromises(); expect(wrapper.get('[role="alert"]').text()).toContain('检查权限'); expect(button(wrapper, '上传文件').attributes('disabled')).toBeDefined();
  });
  it('requires a second delete confirmation and restores focus after a disappeared trigger', async () => {
    const wrapper = await harness(); const trigger = wrapper.get('[aria-label="file-001.txt 操作"]'); (trigger.element as HTMLButtonElement).focus(); await trigger.trigger('click'); await flushPromises(); await wrapper.get('[role="menuitem"]').trigger('keydown', { key: 'End' });
    expect(document.activeElement?.textContent).toBe('删除'); await wrapper.findAll('[role="menuitem"]').find(value => value.text() === '删除')!.trigger('click'); await flushPromises();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true); expect(wrapper.findAll('.file-name').some(value => value.text().includes('file-001.txt'))).toBe(true);
    await button(wrapper, '取消').trigger('click'); await flushPromises(); expect(document.activeElement).toBe(trigger.element);
    await trigger.trigger('click'); await flushPromises(); await wrapper.findAll('[role="menuitem"]').find(value => value.text() === '删除')!.trigger('click'); await flushPromises(); await button(wrapper, '确认删除').trigger('click'); await flushPromises();
    expect(wrapper.findAll('.file-name').some(value => value.text().includes('file-001.txt'))).toBe(false); expect(document.activeElement).toBe(wrapper.get('[aria-label="远程路径"]').element);
  });
  it('supports keyboard menu order, skips disabled items, and restores trigger on Escape', async () => {
    const wrapper = mount(FileMenu, { attachTo: document.body, props: { name: 'folder', disabled: false, downloadable: false } }); wrappers.push(wrapper);
    const trigger = wrapper.get('.file-menu-trigger'); await trigger.trigger('keydown', { key: 'ArrowDown' }); await flushPromises(); expect(document.activeElement?.textContent).toBe('复制路径');
    await wrapper.get('[role="menu"]').trigger('keydown', { key: 'ArrowUp' }); expect(document.activeElement?.textContent).toBe('删除');
    await wrapper.get('[role="menu"]').trigger('keydown', { key: 'ArrowDown' }); expect(document.activeElement?.textContent).toBe('复制路径');
    await wrapper.get('[role="menu"]').trigger('keydown', { key: 'Escape' }); await flushPromises(); expect(document.activeElement).toBe(trigger.element); expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  });
});
