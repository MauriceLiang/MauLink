import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { createIpcClient } from '../src/ipc/client';
import { createSettingsApi } from '../src/ipc/settings';
import { createTerminalPreferences, defaultSettings } from '../src/terminal/preferences';
import { createSettingsMock } from '../src/harness/settings-fixtures';
import { createMockIpc } from '../src/ipc/mock';
import { filterCommands, moveSelection, isTerminalTarget } from '../src/app/palette';
import { locale, messages } from '../src/i18n/locale';
import { shellMessages } from '../src/i18n/shell';
import { filesMessages } from '../src/i18n/files';
import { monitorMessages } from '../src/i18n/monitor';
import { terminalMessages } from '../src/i18n/terminal';
import { connectionMessages } from '../src/i18n/connection';
import { settingsMessages } from '../src/i18n/settings';
import { paletteMessages } from '../src/i18n/palette';
import SettingsDialog from '../src/dialogs/SettingsDialog.vue';
import CommandPalette from '../src/components/base/CommandPalette.vue';
import BaseDropdownMenu from '../src/components/base/BaseDropdownMenu.vue';
import SettingsHarness from '../src/harness/SettingsHarness.vue';
import { applyAccentColor, isValidAccentHex, resolveAccentColor } from '../src/theme/accent';
const wrappers: VueWrapper[] = [];
beforeEach(() => { const values = new Map<string, string>(); vi.stubGlobal('localStorage', { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key,value), clear: () => values.clear(), key: (index: number) => [...values.keys()][index] ?? null, get length() { return values.size; } }); });
function mounted(component: Parameters<typeof mount>[0], props: Record<string, unknown> = {}): VueWrapper { const wrapper = mount(component, { props, attachTo: document.body }); wrappers.push(wrapper); return wrapper; }
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); locale.value = 'zh-CN'; document.body.innerHTML = ''; localStorage.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(element => element.textContent?.trim() === label)!;
async function changeSelect(index: number, label: string) {
  const trigger = document.querySelectorAll<HTMLElement>('[role=combobox]')[index]!;
  trigger.dispatchEvent(new KeyboardEvent('keydown', { key:'ArrowDown', bubbles:true, cancelable:true }));
  await flushPromises();
  const option = [...document.querySelectorAll<HTMLElement>('[role=option]')].find(item => item.textContent?.trim() === label)!;
  option.dispatchEvent(new KeyboardEvent('keydown', { key:'Enter', bubbles:true, cancelable:true }));
  await flushPromises();
}
describe('shared SettingsService preferences', () => {
  it('saves appearance and language with Core revision while preserving terminal and directory token', async () => {
    const value = {...defaultSettings, downloadDirectoryToken: 'opaque-directory-reference'}; const update = vi.fn(payload => ({value: payload.value, revision: 8, updatedAtMs: 2})); const apply = vi.fn();
    const preferences = createTerminalPreferences(createSettingsApi(createIpcClient(createMockIpc({settings_get: () => ({value, revision:7, updatedAtMs:1}), settings_update:update}))), apply);
    await preferences.load(); expect(await preferences.save({theme:'dark', appIconStyle:'dark', language:'en', confirmBeforeDisconnect:false})).toBe(true);
    expect(update.mock.calls[0]![0]).toEqual({expectedRevision:7, value:{...value,theme:'dark',appIconStyle:'dark',language:'en',confirmBeforeDisconnect:false}}); expect(preferences.record.value?.revision).toBe(8); expect(apply).toHaveBeenCalledTimes(2);
    expect(localStorage.length).toBe(0);
  });
  it('retains confirmed settings on revision conflict and supports explicit reload', async () => {
    let conflict = true; const fixture = createSettingsMock({conflict: () => conflict}); const apply = vi.fn(); const preferences = createTerminalPreferences(createSettingsApi(createIpcClient(fixture.transport)), apply);
    await preferences.load(); expect(await preferences.save({theme:'dark'})).toBe(false); expect(preferences.record.value?.value.theme).toBe('system'); expect(fixture.current().revision).toBe(1); expect(apply).toHaveBeenCalledTimes(1); expect(preferences.error.value).toContain('其他操作');
    conflict = false; await preferences.load(); expect(await preferences.save({theme:'dark'})).toBe(true); expect(fixture.current().revision).toBe(2);
  });
  it('does not overwrite settings while a save is pending', async () => {
    let finish!: (value: ReturnType<ReturnType<typeof createSettingsMock>['current']>) => void; const update = vi.fn(() => new Promise<ReturnType<ReturnType<typeof createSettingsMock>['current']>>(resolve => {finish=resolve;}));
    const preferences = createTerminalPreferences(createSettingsApi(createIpcClient(createMockIpc({settings_get:()=>({value:defaultSettings, revision:1,updatedAtMs:1}),settings_update:update}))), () => {});
    await preferences.load(); const saving = preferences.save({theme:'dark'}); expect(await preferences.save({language:'en'})).toBe(false); finish({value:{...defaultSettings,theme:'dark'},revision:2,updatedAtMs:2}); await saving; expect(update).toHaveBeenCalledTimes(1);
  });
  it('does not apply an unsaved dialog draft and saves valid values through IPC', async () => {
    const fixture=createSettingsMock(); const apply=vi.fn(); const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(fixture.transport)),apply); const wrapper=mounted(SettingsDialog,{open:false,preferences}); await wrapper.setProps({open:true}); await flushPromises();
    button('外观').click(); await flushPromises(); await changeSelect(0, '深色'); expect(fixture.current().value.theme).toBe('system');
    document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})); await flushPromises(); expect(fixture.current().value.theme).toBe('dark'); expect(wrapper.emitted('saved')).toHaveLength(1);
  });
  it('persists the selected accent and custom hex color only after saving', async () => {
    const fixture=createSettingsMock(); const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(fixture.transport)),()=>{}); const wrapper=mounted(SettingsDialog,{open:false,preferences}); await wrapper.setProps({open:true}); await flushPromises();
    button('外观').click(); await flushPromises();
    const indigo=document.querySelector<HTMLInputElement>('input[name="accentColor"][value="indigo"]')!; indigo.click(); await flushPromises();
    expect(fixture.current().value.accentColor).toBe('blue');
    document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})); await flushPromises();
    expect(fixture.current().value.accentColor).toBe('indigo');
    await wrapper.setProps({open:false}); await wrapper.setProps({open:true}); await flushPromises(); button('外观').click(); await flushPromises();
    document.querySelector<HTMLInputElement>('input[name="accentColor"][value="custom"]')!.click(); await flushPromises();
    const picker=document.querySelector<HTMLInputElement>('input[type="color"]')!; picker.value='#12abef'; picker.dispatchEvent(new Event('input',{bubbles:true})); await flushPromises();
    document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})); await flushPromises();
    expect(fixture.current().value).toMatchObject({accentColor:'custom',customAccentColor:'#12abef'});
  });
  it('saves the icon independently of theme, restores it on reopen, and discards cancellation', async () => {
    const fixture=createSettingsMock(); const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(fixture.transport)),()=>{}); const wrapper=mounted(SettingsDialog,{open:false,preferences}); await wrapper.setProps({open:true}); await flushPromises();
    button('外观').click(); await flushPromises();
    await changeSelect(1, '深色'); await flushPromises();
    expect(document.querySelector('img[alt="应用图标预览"]')?.getAttribute('src')).toContain('app-icon-dark');
    expect(fixture.current().value.appIconStyle).toBe('light');
    button('取消').click(); await wrapper.setProps({open:false}); await wrapper.setProps({open:true}); await flushPromises();
    expect(document.querySelectorAll('[role=combobox]')[1]!.textContent).toContain('浅色');
    await changeSelect(1, '深色');
    document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})); await flushPromises();
    expect(fixture.current().value.appIconStyle).toBe('dark'); expect(fixture.current().value.theme).toBe('system'); expect(wrapper.emitted('saved')).toHaveLength(1);
    await wrapper.setProps({open:false}); await wrapper.setProps({open:true}); await flushPromises();
    expect(document.querySelectorAll('[role=combobox]')[1]!.textContent).toContain('深色');
    await changeSelect(0, '深色'); await changeSelect(1, '浅色');
    document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})); await flushPromises();
    expect(fixture.current().value.theme).toBe('dark'); expect(fixture.current().value.appIconStyle).toBe('light');
  });
  it('reports a committed setting whose native icon failed and reloads the saved revision', async () => {
    let stored={value:{...defaultSettings},revision:1,updatedAtMs:1};
    const transport=createMockIpc({settings_get:()=>structuredClone(stored),settings_update:payload=>{
      stored={value:payload.value,revision:2,updatedAtMs:2};
      throw {code:'INTERNAL',messageKey:'errors.appIconApplyFailed',params:{},retryable:false,action:'none',stage:null,requestId:null,details:null};
    }});
    const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(transport)),()=>{});
    await preferences.load(); expect(await preferences.save({appIconStyle:'dark'})).toBe(false);
    expect(preferences.error.value).toContain('设置已保存，但应用图标更新失败');
    expect(preferences.record.value?.revision).toBe(1);
    await preferences.load(); expect(preferences.record.value?.revision).toBe(2); expect(preferences.record.value?.value.appIconStyle).toBe('dark');
  });
  it('keeps settings reload reachable after a read failure', async () => {
    const fixture=createSettingsMock({readFailure:()=>true}); const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(fixture.transport)),()=>{}); mounted(SettingsDialog,{open:true,preferences});
    // Opening after mount follows the same dialog lifecycle as the application.
    await preferences.load(); await flushPromises(); expect(document.querySelector('[role="alert"]')).not.toBeNull(); expect(button('重新加载设置（放弃修改）').disabled).toBe(false);
  });
});
describe('catalogs and command palette', () => {
  it('accepts only six-digit hex accent colors and falls back safely for invalid values', () => {
    expect(isValidAccentHex('#12aBeF')).toBe(true); expect(isValidAccentHex('rgb(1,2,3)')).toBe(false); expect(isValidAccentHex('#12345678')).toBe(false);
    expect(resolveAccentColor('custom','#12aBeF')).toBe('#12aBeF'); expect(resolveAccentColor('custom','var(--color-primary)')).toBe('#3B82F6');
    const root=document.createElement('div'); applyAccentColor('purple',null,root); expect(root.style.getPropertyValue('--color-accent-base')).toBe('#A855F7');
  });
  it('has complete bilingual entries and matching interpolation parameters by module', () => {
    for (const catalog of [shellMessages,filesMessages,monitorMessages,terminalMessages,connectionMessages,settingsMessages,paletteMessages]) for (const [zh,en] of Object.values(catalog)) { expect(zh.length).toBeGreaterThan(0); expect(en.length).toBeGreaterThan(0); expect([...zh.matchAll(/\{(\w+)\}/g)].map(value=>value[1]).sort()).toEqual([...en.matchAll(/\{(\w+)\}/g)].map(value=>value[1]).sort()); }
    locale.value='en'; expect(messages(filesMessages)('deleteNote',{name:'服务器.txt'})).toContain('服务器.txt'); expect(messages(shellMessages)('oneTerminal')).toBe('1 terminal');
  });
  it('filters normalized terms and skips every disabled option in both directions', () => {
    const values=[{id:'a',label:'Alpha',disabled:true},{id:'b',label:'Beta',meta:'root@host',keywords:['服务器']},{id:'c',label:'Gamma',disabled:true}]; expect(filterCommands(values,'ＢＥＴＡ host')).toEqual([values[1]]); expect(filterCommands(values,'服务器')).toEqual([values[1]]); expect(moveSelection(values,1,1)).toBe(1); expect(moveSelection(values,1,-1)).toBe(1); expect(moveSelection(values.map(value=>({...value,disabled:true})),0,1)).toBe(-1); expect(moveSelection([],0,1)).toBe(-1);
  });
  it('focuses search, moves selection, executes Enter, closes with Esc, and restores trigger', async () => {
    const trigger=document.createElement('button');document.body.append(trigger);trigger.focus();const wrapper=mounted(CommandPalette,{open:false,commands:[{id:'disabled',label:'Disabled',disabled:true},{id:'one',label:'One'},{id:'two',label:'Two'}]}); await wrapper.setProps({open:true});await flushPromises();
    const input=document.querySelector<HTMLInputElement>('[role="combobox"]')!;expect(document.activeElement).toBe(input); const selected=()=>document.querySelector('[role="option"][aria-selected="true"]')?.textContent?.trim();expect(selected()).toBe('One');const scrolling=vi.fn(); Object.defineProperty(HTMLElement.prototype,'scrollIntoView',{value:scrolling,configurable:true}); input.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowUp',bubbles:true}));await flushPromises();expect(selected()).toBe('Two');input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));expect(wrapper.emitted('execute')).toEqual([['two']]);input.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));expect(wrapper.emitted('close')).toHaveLength(1);await wrapper.setProps({open:false});await flushPromises();expect(document.activeElement).toBe(trigger);
  });
  it('shows no matches without executing or focusing a disabled result', async () => {
    const wrapper=mounted(CommandPalette,{open:true,commands:[{id:'off',label:'Disabled',disabled:true}]});await flushPromises();const input=document.querySelector<HTMLInputElement>('[role="combobox"]')!; input.value='missing'; input.dispatchEvent(new Event('input',{bubbles:true})); await flushPromises(); input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));expect(document.querySelector('[role="status"]')?.textContent).toBe('没有匹配的命令');expect(wrapper.emitted('execute')).toBeUndefined();
  });
  it('leaves terminal Ctrl/Cmd K intact while ordinary UI opens the palette', async () => {
    mounted(SettingsHarness);await flushPromises();const term=document.createElement('div');term.className='xterm';const input=document.createElement('textarea');term.append(input);document.body.append(term);expect(isTerminalTarget(input)).toBe(true);input.focus();const remote=new KeyboardEvent('keydown',{key:'k',ctrlKey:true,bubbles:true,cancelable:true});input.dispatchEvent(remote);await flushPromises();expect(remote.defaultPrevented).toBe(false);expect(document.querySelector('[role="dialog"]')).toBeNull();
    const normal=new KeyboardEvent('keydown',{key:'k',metaKey:true,bubbles:true,cancelable:true});document.querySelector('input')!.dispatchEvent(normal);await flushPromises();expect(normal.defaultPrevented).toBe(true);expect(document.activeElement?.getAttribute('role')).toBe('combobox');
  });
  it('uses one menu with disabled skipping, Esc focus restore and action dispatch', async () => {
    const wrapper=mounted(BaseDropdownMenu,{label:'Actions',items:[{id:'off',label:'Unavailable',disabled:true},{id:'first',label:'First'},{id:'last',label:'Last'}]});const trigger=wrapper.get('button');const menu=()=>({trigger:async(_event:string,options:{key:string})=>{document.querySelector('[role=menu]')!.dispatchEvent(new KeyboardEvent('keydown',{...options,bubbles:true,cancelable:true}));await new Promise(resolve=>setTimeout(resolve,0));await flushPromises();}});trigger.element.focus();await trigger.trigger('keydown',{key:'ArrowDown'});await flushPromises();await new Promise(resolve=>setTimeout(resolve,0));expect(document.activeElement?.textContent).toBe('First');await menu().trigger('keydown',{key:'End'});expect(document.activeElement?.textContent).toBe('Last');await menu().trigger('keydown',{key:'Escape'});await new Promise(resolve=>setTimeout(resolve,0));expect(document.activeElement).toBe(trigger.element);await trigger.trigger('click');await flushPromises();document.querySelectorAll<HTMLElement>('[role=menuitem]')[1]!.click(); await new Promise(resolve=>setTimeout(resolve,0));await flushPromises();expect(wrapper.emitted('action')).toEqual([['first']]);
  });
});
