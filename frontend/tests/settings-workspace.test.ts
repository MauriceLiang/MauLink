import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import SettingsWorkspace from '../src/app/settings/SettingsWorkspace.vue';
import { createIpcClient } from '../src/ipc/client';
import { createMockIpc } from '../src/ipc/mock';
import { createSettingsApi } from '../src/ipc/settings';
import type { BackgroundImagesApi } from '../src/ipc/background-images';
import type { AppSettings } from '../../contracts/v1/AppSettings';
import type { SettingsRecord } from '../../contracts/v1/SettingsRecord';
import { defaultSettings, createTerminalPreferences } from '../src/terminal/preferences';
import { fixtureError } from '../src/harness/server-fixtures';

const wrappers: VueWrapper[] = [];
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = '';
});

function settingsFixture(options: { conflict?: () => boolean; changed?: (record: SettingsRecord) => void } = {}) {
  let record: SettingsRecord = { value: structuredClone(defaultSettings), revision: 1, updatedAtMs: 1 };
  const update = vi.fn((payload: { expectedRevision: number; value: AppSettings }) => {
    if (options.conflict?.() || payload.expectedRevision !== record.revision) throw fixtureError('REVISION_CONFLICT', 'errors.revisionConflict');
    record = { value: structuredClone(payload.value), revision: record.revision + 1, updatedAtMs: 2 };
    options.changed?.(record);
    return structuredClone(record);
  });
  const transport = createMockIpc({ settings_get: () => structuredClone(record), settings_update: update });
  const client = createIpcClient(transport);
  const apply = vi.fn();
  const preferences = createTerminalPreferences(createSettingsApi(client), apply);
  return { current: () => structuredClone(record), update, apply, preferences };
}

function mountWorkspace(section: 'general' | 'security' | 'appearance' | 'files' | 'network' | 'terminal' | 'language' = 'appearance', options: { conflict?: () => boolean; backgroundImages?: BackgroundImagesApi } = {}) {
  const fixture = settingsFixture(options);
  const backgroundImages = options.backgroundImages ?? {
    select: vi.fn(async () => null), get: vi.fn(), resolve: vi.fn(), delete: vi.fn(async () => {}),
  } as unknown as BackgroundImagesApi;
  const wrapper = mount(SettingsWorkspace, { attachTo: document.body, props: { active: true, section, preferences: fixture.preferences, backgroundImages } });
  wrappers.push(wrapper);
  return { wrapper, fixture, backgroundImages };
}

const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(element => element.textContent?.trim() === label)!;
async function changeSelect(label: string, value: string) {
  const field = [...document.querySelectorAll<HTMLElement>('.base-field')].find(element => element.querySelector('label')?.textContent?.trim() === label)!;
  field.querySelector<HTMLElement>('[role="combobox"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
  await flushPromises();
  const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(element => element.textContent?.trim() === value)!;
  option.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  await flushPromises();
}

describe('settings workspace page', () => {
  it('keeps a regular preference draft across sections and saves one expected-revision snapshot', async () => {
    const { wrapper, fixture } = mountWorkspace();
    await flushPromises();
    expect(wrapper.get('h1').text()).toBe('外观');
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    await changeSelect('主题', '深色');
    expect(fixture.current().value.theme).toBe('system');
    expect(button('保存更改').disabled).toBe(false);

    await wrapper.setProps({ section: 'language' });
    await flushPromises();
    await wrapper.setProps({ section: 'appearance' });
    await flushPromises();
    expect(wrapper.get('[role="combobox"]').text()).toContain('深色');
    await changeSelect('界面密度', '紧凑');
    button('保存更改').click();
    await flushPromises();

    expect(fixture.update).toHaveBeenCalledExactlyOnceWith({
      expectedRevision: 1,
      value: { ...defaultSettings, theme: 'dark', uiDensity: 'compact' },
    });
    expect(fixture.current()).toMatchObject({ revision: 2, value: { theme: 'dark', uiDensity: 'compact' } });
    expect(button('保存更改').disabled).toBe(true);
  });

  it('discards edits and offers save, discard, or continue when leaving a dirty page', async () => {
    const { wrapper, fixture } = mountWorkspace();
    await flushPromises();
    await changeSelect('主题', '深色');
    await (wrapper.vm as unknown as { requestLeave: () => void }).requestLeave();
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('常规设置更改');
    expect(button('继续编辑')).toBeDefined();
    expect(button('放弃并离开')).toBeDefined();
    expect(button('保存并离开')).toBeDefined();
    button('继续编辑').click();
    await flushPromises();
    expect(wrapper.emitted('leave')).toBeUndefined();

    await (wrapper.vm as unknown as { requestLeave: () => void }).requestLeave();
    await flushPromises();
    button('放弃并离开').click();
    await flushPromises();
    expect(wrapper.emitted('leave')).toHaveLength(1);
    expect(fixture.current().value.theme).toBe('system');
  });

  it('keeps a failed revision-conflict draft until the user confirms reload', async () => {
    let conflict = true;
    const { wrapper, fixture } = mountWorkspace('appearance', { conflict: () => conflict });
    await flushPromises();
    await changeSelect('主题', '深色');
    button('保存更改').click();
    await flushPromises();
    expect(fixture.current().value.theme).toBe('system');
    expect(button('保存更改').disabled).toBe(false);
    expect(button('重新加载设置（放弃修改）')).toBeDefined();
    button('重新加载设置（放弃修改）').click();
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('放弃当前所有未保存');
    conflict = false;
    button('重新加载').click();
    await flushPromises();
    expect(wrapper.get('[role="combobox"]').text()).toContain('跟随系统');
    expect(button('保存更改').disabled).toBe(true);
  });

  it('edits terminal settings in the page and commits them through the shared settings revision', async () => {
    const { wrapper, fixture } = mountWorkspace('terminal');
    await flushPromises();
    expect(wrapper.get('h1').text()).toBe('终端');
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    const fontSize = document.querySelector<HTMLInputElement>('.terminal-settings-row input[type="number"]')!;
    await wrapper.get('.terminal-settings-row input[type="number"]').setValue('18');
    await flushPromises();
    expect(button('保存更改').disabled).toBe(false);
    button('保存更改').click();
    await flushPromises();
    expect(fixture.current().value.terminalFontSize).toBe(18);
    expect(fixture.update).toHaveBeenCalledWith(expect.objectContaining({ expectedRevision: 1, value: expect.objectContaining({ terminalFontSize: 18 }) }));
    expect(fontSize.value).toBe('18');
  });

  it('removes an imported terminal background image when the page draft is discarded', async () => {
    const asset = { id: 'fixture-image', fileName: 'fixture.png', mediaType: 'image/png' as const, width: 10, height: 10, byteLength: 100, createdAtMs: 1 };
    const images = { select: vi.fn(async () => asset), get: vi.fn(), resolve: vi.fn(async () => ({ asset, src: 'asset://fixture-image' })), delete: vi.fn(async () => {}) } as unknown as BackgroundImagesApi;
    const { fixture } = mountWorkspace('terminal', { backgroundImages: images });
    await flushPromises();
    document.querySelector<HTMLInputElement>('input[name="terminalThemeMode"][value="image"]')!.click();
    await flushPromises();
    button('选择图片').click();
    await flushPromises();
    expect(document.querySelector('.terminal-background-preview-image')).not.toBeNull();
    expect(button('保存更改').disabled).toBe(false);
    button('放弃更改').click();
    await flushPromises();
    expect(images.delete).toHaveBeenCalledExactlyOnceWith('fixture-image');
    expect(fixture.current().value.terminalBackgroundImage.imageId).toBeNull();
    expect(button('保存更改').disabled).toBe(true);
  });
});
