import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import type { GeoIpDatabaseStatus } from '../../contracts/v1/GeoIpDatabaseStatus';
import GeoIpSettingsSection from '../src/components/server-overview/GeoIpSettingsSection.vue';
import SettingsDialog from '../src/dialogs/SettingsDialog.vue';
import { createIpcClient } from '../src/ipc/client';
import { createGeoIpApi } from '../src/ipc/geoip';
import { createMockIpc } from '../src/ipc/mock';
import { createSettingsApi } from '../src/ipc/settings';
import { createTerminalPreferences, defaultSettings } from '../src/terminal/preferences';
import type { BackgroundImagesApi } from '../src/ipc/background-images';

const empty: GeoIpDatabaseStatus = { location: null, asn: null, automaticUpdates: false, updateIntervalDays: 30, lastCheckedAtMs: null, lastError: null };
const installed: GeoIpDatabaseStatus = { ...empty, automaticUpdates: true, location: { fileName: 'City.mmdb', databaseType: 'DBIP-City-Lite', buildAtMs: 1790812800000, source: 'dbIp', available: true } };
const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ''; });
function mountSection(api: ReturnType<typeof createGeoIpApi>) {
  const wrapper = mount(GeoIpSettingsSection, { props: { api }, attachTo: document.body });
  wrappers.push(wrapper); return wrapper;
}
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(element => element.textContent?.trim() === label)!;

describe('offline GeoIP settings', () => {
  it('installs databases and persists a selected update frequency through typed IPC', async () => {
    const update = vi.fn(() => installed);
    const configure = vi.fn(({ updateIntervalDays }) => ({ ...installed, updateIntervalDays }));
    const wrapper = mountSection(createGeoIpApi(createIpcClient(createMockIpc({ geoip_database_get: () => empty, geoip_database_update: update, geoip_database_configure: configure }))));
    await flushPromises();
    expect(button('删除数据库').disabled).toBe(true);
    button('一键下载配置').click(); await flushPromises();
    expect(update).toHaveBeenCalledExactlyOnceWith({});
    expect(wrapper.text()).toContain('City.mmdb');
    document.querySelector<HTMLElement>('[role=combobox]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
    await flushPromises();
    const option = [...document.querySelectorAll<HTMLElement>('[role=option]')].find(value => value.textContent?.trim() === '每周')!;
    option.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })); await flushPromises();
    expect(configure).toHaveBeenCalledExactlyOnceWith({ updateIntervalDays: 7 });
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('uses a purpose-bound file token, handles selection cancellation, and deletes app databases', async () => {
    const select = vi.fn().mockResolvedValueOnce(null).mockResolvedValueOnce({ token: 'mmdb-token', purpose: 'geoIpDatabase', displayName: 'City.mmdb', expiresAtMs: 4102444800000 });
    const importFile = vi.fn(() => ({ ...installed, automaticUpdates: false }));
    const deleteDatabases = vi.fn(() => empty);
    const wrapper = mountSection(createGeoIpApi(createIpcClient(createMockIpc({ geoip_database_get: () => empty, local_file_select: select, geoip_database_import: importFile, geoip_database_delete: deleteDatabases }))));
    await flushPromises(); button('导入 MMDB 文件').click(); await flushPromises();
    expect(importFile).not.toHaveBeenCalled(); expect(wrapper.emitted('changed')).toBeUndefined();
    button('导入 MMDB 文件').click(); await flushPromises();
    expect(select).toHaveBeenLastCalledWith({ purpose: 'geoIpDatabase' });
    expect(importFile).toHaveBeenCalledExactlyOnceWith({ token: 'mmdb-token' });
    expect(wrapper.text()).toContain('不自动替换文件');
    button('删除数据库').click(); await flushPromises();
    expect(deleteDatabases).toHaveBeenCalledExactlyOnceWith({});
    expect(button('删除数据库').disabled).toBe(true);
  });

  it('reloads confirmed status after download failure and keeps existing databases visible', async () => {
    const get = vi.fn(() => installed);
    const update = vi.fn(() => { throw { code: 'INTERNAL', messageKey: 'errors.geoIpDatabaseDownloadFailed', params: {}, retryable: true, action: 'retry', details: null, stage: null, requestId: null }; });
    const wrapper = mountSection(createGeoIpApi(createIpcClient(createMockIpc({ geoip_database_get: get, geoip_database_update: update }))));
    await flushPromises(); button('立即检查更新').click(); await flushPromises();
    expect(wrapper.text()).toContain('数据库下载失败'); expect(wrapper.text()).toContain('City.mmdb');
    expect(get).toHaveBeenCalledTimes(2); expect(button('立即检查更新').disabled).toBe(false);
  });

  it('keeps the settings dialog open and prevents switching sections during an update', async () => {
    let finish!: (value: GeoIpDatabaseStatus) => void;
    const transport = createMockIpc({ geoip_database_get: () => empty, geoip_database_update: () => new Promise<GeoIpDatabaseStatus>(resolve => { finish = resolve; }), settings_get: () => ({ value: defaultSettings, revision: 1, updatedAtMs: 1 }) });
    const client = createIpcClient(transport);
    const preferences = createTerminalPreferences(createSettingsApi(client), () => {});
    const wrapper = mount(SettingsDialog, { props: { open: false, preferences, backgroundImages: {} as BackgroundImagesApi, geoip: createGeoIpApi(client), initialSection: 'network' }, attachTo: document.body }); wrappers.push(wrapper);
    await wrapper.setProps({ open: true }); await flushPromises(); button('一键下载配置').click(); await flushPromises();
    expect(button('通用').disabled).toBe(true); expect(button('完成').disabled).toBe(true);
    expect(document.querySelector<HTMLButtonElement>('[aria-label="关闭对话框"]')!.disabled).toBe(true); expect(button('保存设置')).toBeUndefined();
    document.querySelector('[role=dialog]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); await flushPromises();
    expect(wrapper.emitted('close')).toBeUndefined();
    finish(installed); await flushPromises();
    expect(button('完成').disabled).toBe(false); expect(wrapper.emitted('databaseChanged')).toHaveLength(1);
    button('完成').click(); await flushPromises(); expect(wrapper.emitted('close')).toHaveLength(1);
  });
});
