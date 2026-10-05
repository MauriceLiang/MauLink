import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../contracts/v1/TerminalChunk';
import type { TerminalSnapshot } from '../../contracts/v1/TerminalSnapshot';
import { createIpcClient } from '../src/ipc/client';
import { createMockIpc } from '../src/ipc/mock';
import { createTerminalApi } from '../src/ipc/terminal';
import { createSettingsApi } from '../src/ipc/settings';
import { createTerminalController } from '../src/terminal/controller';
import { createTerminalPreferences, defaultSettings } from '../src/terminal/preferences';
import { createOutputConsumer, encodeBytesBase64, decodeBase64, nextSequence } from '../src/terminal/codec';
const renderers = vi.hoisted(() => {
  class Renderer {
    options: Record<string, unknown>; cols = 80; rows = 24; output: Uint8Array[] = []; disposed = false; focus = vi.fn(); clear = vi.fn();
    data: (value: string) => void = () => {}; binary: (value: string) => void = () => {}; selected: () => void = () => {}; selection = '';
    constructor(options: Record<string, unknown>) { this.options = options; instances.push(this); }
    loadAddon() {} open() {} dispose() { this.disposed = true; }
    write(bytes: Uint8Array | string, done?: () => void) { if (bytes instanceof Uint8Array) this.output.push(bytes); done?.(); }
    onData(handler: (value: string) => void) { this.data = handler; } onBinary(handler: (value: string) => void) { this.binary = handler; }
    onResize() {} onSelectionChange(handler: () => void) { this.selected = handler; } getSelection() { return this.selection; }
  }
  const instances: Renderer[] = []; return { Renderer, instances };
});
vi.mock('@xterm/xterm', () => ({ Terminal: renderers.Renderer }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }));
const controllers: ReturnType<typeof createTerminalController>[] = [];
beforeEach(() => { const values = new Map<string,string>(); vi.stubGlobal('localStorage', { getItem: (key:string)=>values.get(key)??null, setItem: (key:string,value:string)=>values.set(key,value), clear: ()=>values.clear() }); });
afterEach(() => { controllers.splice(0).forEach(controller => controller.dispose()); renderers.instances.splice(0); vi.restoreAllMocks(); vi.useRealTimers(); document.body.innerHTML = ''; vi.unstubAllGlobals(); });
const chunk = (seq = '1', bytes = new TextEncoder().encode('hello')): TerminalChunk => ({ terminalId: 'terminal-a', streamId: 'stream-a', seq, byteLength: bytes.length, dataBase64: encodeBytesBase64(bytes) });
const snapshot = (state: TerminalSnapshot['state'] = 'running'): TerminalSnapshot => ({ terminalId: 'terminal-a', streamId: 'stream-a', connectionId: 'connection-a', state, size: { columns: 80, rows: 24, pixelWidth: null, pixelHeight: null }, exitStatus: null, exitSignal: null, error: null, createdAtMs: 1, updatedAtMs: 1 });
function controllerFor(handlers: Parameters<typeof createMockIpc>[0] = {}) {
  const channel = { onmessage: (_chunk: TerminalChunk) => {} } as Channel<TerminalChunk>;
  const open = vi.fn(() => ({ terminalId: 'terminal-a', streamId: 'stream-a' })); const close = vi.fn(() => snapshot('closed')); const ack = vi.fn(); const write = vi.fn(payload => ({ inputSeq: payload.inputSeq, duplicate: false }));
  const controller = createTerminalController(createTerminalApi(createIpcClient(createMockIpc({ terminal_open: open, terminal_get: () => snapshot(), terminal_close: close, terminal_ack: ack, terminal_resize: payload => payload, terminal_write: write, ...handlers }))), () => channel);
  controllers.push(controller); return { controller, channel, open, close, ack, write };
}
async function attach(controller: ReturnType<typeof createTerminalController>) {
  vi.spyOn(HTMLElement.prototype,'getBoundingClientRect').mockReturnValue({ width: 800, height: 600 } as DOMRect);
  const host = document.createElement('div'); document.body.append(host); const id = controller.create('connection-a', 'server-a'); await controller.attach(id, host); return { id, host, renderer: renderers.instances.at(-1)! };
}
describe('terminal byte flow', () => {
  it('increments decimal u64 sequences without Number precision loss', () => { expect(nextSequence('9007199254740999')).toBe('9007199254741000'); expect(nextSequence('99')).toBe('100'); });
  it('acknowledges only after xterm write callback and preserves UTF-8/ANSI bytes', async () => {
    let done!: () => void; const ack = vi.fn(async () => {}); const bytes = Uint8Array.of(0xe4,0xb8,0xad,27,91,51,49,109);
    const write = vi.fn((_bytes: Uint8Array, callback: () => void) => { done = callback; }); const consume = createOutputConsumer(write,ack);
    const pending = consume(chunk('1',bytes)); await flushPromises(); expect(ack).not.toHaveBeenCalled(); expect(write.mock.calls[0]?.[0]).toEqual(bytes);
    done(); await pending; expect(ack).toHaveBeenCalledWith(chunk('1',bytes));
  });
  it('rejects sequence gaps and wrong byte length without ACK', async () => {
    const ack = vi.fn(async () => {}); const consume = createOutputConsumer((_bytes,done) => done(),ack);
    await expect(consume(chunk('2'))).rejects.toThrow('序号'); expect(ack).not.toHaveBeenCalled();
    const other = createOutputConsumer((_bytes,done) => done(),ack); await expect(other({ ...chunk(),byteLength:3 })).rejects.toThrow('长度'); expect(ack).not.toHaveBeenCalled();
  });
  it('bounds queued output at the existing 128 KiB window', async () => {
    let callback!: () => void; const consume = createOutputConsumer((_bytes,done) => { callback = done; },async () => {});
    const queued = [1,2,3,4].map(value => consume(chunk(String(value),new Uint8Array(32768))).catch(() => undefined));
    await flushPromises(); await expect(consume(chunk('5',new Uint8Array(1)))).rejects.toThrow('流控'); callback(); await Promise.all(queued);
  });
  it('accepts the next Core window while a processed ACK response is still in transit', async () => {
    let reply!: () => void;
    const ack = vi.fn(() => new Promise<void>(resolve => { reply = resolve; }));
    const consume = createOutputConsumer((_bytes, done) => done(), ack);
    const first = consume(chunk('1', new Uint8Array(32768))); await flushPromises();
    const next = [2,3,4,5].map(seq => consume(chunk(String(seq), new Uint8Array(32768))));
    reply(); await first;
    for (const pending of next) { await flushPromises(); reply(); await pending; }
    expect(ack).toHaveBeenCalledTimes(5);
  });
  it('keeps large contiguous stdout out of reactive metadata', async () => {
    const {controller,channel,ack} = controllerFor(); const {renderer} = await attach(controller); const metadata = controller.tabs.value;
    for (let seq=1;seq<=128;seq++) { channel.onmessage(chunk(String(seq),new Uint8Array(8192))); await flushPromises(); }
    expect(ack).toHaveBeenCalledTimes(128); expect(renderer.output.reduce((size,bytes)=>size+bytes.length,0)).toBe(1048576); expect(controller.tabs.value).toBe(metadata);
    expect(Object.keys(metadata[0]!)).not.toContain('stdout');
  });
  it('buffers pre-open Channel output, then validates the opened stream and ACKs', async () => {
    let resolve!: (value: { terminalId: string; streamId: string }) => void;
    const {controller,channel,ack} = controllerFor({ terminal_open: () => new Promise(done => { resolve = done; }) });
    const host=document.createElement('div'); document.body.append(host); const id=controller.create('connection-a', 'server-a'); const opening=controller.attach(id,host);
    channel.onmessage(chunk()); expect(ack).not.toHaveBeenCalled(); resolve({terminalId:'terminal-a',streamId:'stream-a'}); await opening; await flushPromises(); expect(ack).toHaveBeenCalledOnce();
  });
  it('halts mismatched output streams and closes that remote PTY explicitly', async () => {
    const {controller,channel,close,ack} = controllerFor(); await attach(controller); channel.onmessage({...chunk(),streamId:'other'}); await flushPromises();
    expect(close).toHaveBeenCalledWith({terminalId:'terminal-a'}); expect(ack).not.toHaveBeenCalled();
  });
  it('detach and remount keep the same xterm, PTY and output consumer', async () => {
    const {controller,channel,close,open,ack}=controllerFor(); const {id,renderer}=await attach(controller); controller.detach(id);
    channel.onmessage(chunk()); await flushPromises(); const host=document.createElement('div');document.body.append(host);await controller.attach(id,host);
    expect(close).not.toHaveBeenCalled();expect(open).toHaveBeenCalledOnce();expect(renderers.instances).toHaveLength(1);expect(renderer.disposed).toBe(false);expect(ack).toHaveBeenCalledOnce();
  });
  it('forwards Ctrl+C, Unicode and binary mouse bytes with ordered decimal input sequence', async () => {
    const {controller,write}=controllerFor();const {renderer}=await attach(controller);renderer.data('\x03');renderer.data('中文');renderer.binary('\xff');await flushPromises();
    expect(write.mock.calls.map(([payload])=>payload.inputSeq)).toEqual(['1','2','3']);expect(decodeBase64(write.mock.calls[0]![0].dataBase64)).toEqual(Uint8Array.of(3));expect(decodeBase64(write.mock.calls[2]![0].dataBase64)).toEqual(Uint8Array.of(255));
  });
  it('splits large paste at 64 KiB and rejects additional paste past the 256 KiB input budget', async () => {
    let resolve!: (value: {inputSeq:string;duplicate:boolean})=>void;const write=vi.fn(payload=>new Promise<{inputSeq:string;duplicate:boolean}>(done=>{resolve=()=>done({inputSeq:payload.inputSeq,duplicate:false});}));
    const {controller}=controllerFor({terminal_write:write});const {renderer}=await attach(controller);renderer.data('x'.repeat(262144));renderer.data('y');await flushPromises();expect(controller.tabs.value[0]?.error).toContain('暂存已满');
    for(let index=0;index<4;index++){resolve({inputSeq:String(index+1),duplicate:false});await flushPromises();}
    expect(write).toHaveBeenCalledTimes(4);expect(write.mock.calls.every(([payload])=>decodeBase64(payload.dataBase64).length===65536)).toBe(true);
  });
  it('close failure preserves the renderer/tab and does not pretend PTY closed', async () => {
    const {controller}=controllerFor({terminal_close:()=>{throw new Error('raw diagnostic');}});const {id,renderer}=await attach(controller);
    expect(await controller.close(id)).toBe(false);expect(controller.tabs.value[0]?.state).toBe('running');expect(renderer.disposed).toBe(false);expect(controller.tabs.value[0]?.error).not.toContain('raw');
  });
  it('explicit close succeeds before removing the tab and disposing xterm', async () => {
    const {controller,close}=controllerFor();const {id,renderer}=await attach(controller);expect(await controller.close(id)).toBe(true);expect(close).toHaveBeenCalledOnce();expect(controller.tabs.value).toHaveLength(0);expect(renderer.disposed).toBe(true);
  });
  it('applies settings to all runtimes and uses opt-in selection copying', async () => {
    vi.useFakeTimers();const {controller}=controllerFor();const {renderer}=await attach(controller);const clipboard=vi.fn(async()=>{});vi.stubGlobal('navigator',{clipboard:{writeText:clipboard}});
    renderer.selection='fixture selection';renderer.selected();await vi.advanceTimersByTimeAsync(150);expect(clipboard).not.toHaveBeenCalled();
    controller.setCopyPreference(()=>true);renderer.selected();await vi.advanceTimersByTimeAsync(150);expect(clipboard).toHaveBeenCalledWith('fixture selection');
    controller.applySettings({...defaultSettings,terminalFontSize:18,terminalCursorStyle:'bar',terminalScrollbackLines:12000});expect(renderer.options.fontSize).toBe(18);expect(renderer.options.cursorStyle).toBe('bar');
  });
  it('updates theme, line height and cursor behavior on existing xterm instances without reopening the PTY', async () => {
    const {controller,open}=controllerFor();const {renderer}=await attach(controller);
    controller.applySettings({...defaultSettings,terminalThemeMode:'customColor',terminalCustomColors:{background:'#102030',foreground:'#E0E0E0',cursor:'#33AAFF',selection:'#7755CC'},terminalLineHeight:1.5,terminalCursorBlink:false});
    expect(renderer.options.theme).toMatchObject({background:'#102030',foreground:'#E0E0E0',cursor:'#33AAFF'});
    expect(renderer.options.lineHeight).toBe(1.5);expect(renderer.options.cursorBlink).toBe(false);
    expect(document.documentElement.style.getPropertyValue('--color-terminal-bg')).toBe('#102030');expect(open).toHaveBeenCalledOnce();
  });
  it('keeps opaque terminals intact when enabling images and makes new terminals transparent', async () => {
    const {controller,open,close}=controllerFor(); const {renderer:existing}=await attach(controller);
    const previousBackground=(existing.options.theme as {background:string}).background;
    controller.applySettings({...defaultSettings,terminalThemeMode:'image',terminalBackgroundImage:{...defaultSettings.terminalBackgroundImage,imageId:'00000000-0000-4000-8000-000000000001'}});
    expect(existing.options.theme).toMatchObject({background:previousBackground});
    expect(controller.requiresReopen.value).toBe(true);
    const nextId=controller.create('connection-a', 'server-a'); const host=document.createElement('div'); document.body.append(host); await controller.attach(nextId,host);
    const next=renderers.instances.at(-1)!;
    expect(next.options.allowTransparency).toBe(true);
    expect(next.options.theme).toMatchObject({background:'transparent'});
    expect(controller.requiresReopen.value).toBe(true);
    expect(open).toHaveBeenCalledTimes(2); expect(close).not.toHaveBeenCalled();
    controller.applySettings({...defaultSettings,terminalThemeMode:'dark'});
    expect((existing.options.theme as {background:string}).background).toBe('#111318');
    expect((next.options.theme as {background:string}).background).toBe('#111318');
    expect(controller.requiresReopen.value).toBe(false);
    expect(open).toHaveBeenCalledTimes(2); expect(close).not.toHaveBeenCalled();
  });
  it('updates Follow App colors when the system theme changes without reopening the PTY', async () => {
    let listener: ((event: MediaQueryListEvent) => void) | undefined;
    const query={matches:false,addEventListener:(_type:string,callback:EventListenerOrEventListenerObject)=>{listener=callback as (event:MediaQueryListEvent)=>void;},removeEventListener:vi.fn()} as unknown as MediaQueryList;
    vi.stubGlobal('matchMedia',vi.fn(()=>query));
    const {controller,open}=controllerFor();const {renderer}=await attach(controller);
    expect((renderer.options.theme as {background:string}).background).toBe('#FFFFFF');
    Object.defineProperty(query,'matches',{value:true,configurable:true});listener?.({matches:true} as MediaQueryListEvent);
    expect((renderer.options.theme as {background:string}).background).toBe('#111318');
    expect(open).toHaveBeenCalledOnce();
  });
});
describe('terminal preferences',()=>{
  it('preserves unrelated settings and revisions, defaults copy-on-select off',async()=>{
    const value={...defaultSettings,language:'en' as const,theme:'dark' as const};const update=vi.fn(payload=>({value:payload.value,revision:8,updatedAtMs:2}));const apply=vi.fn();
    const preferences=createTerminalPreferences(createSettingsApi(createIpcClient(createMockIpc({settings_get:()=>({value,revision:7,updatedAtMs:1}),settings_update:update}))),apply);
    expect(preferences.copyOnSelect.value).toBe(false);await preferences.load();await preferences.save({terminalFontFamily:'monospace',terminalFontSize:18,terminalCursorStyle:'block',terminalScrollbackLines:10000},true);
    expect(update.mock.calls[0]![0]).toMatchObject({expectedRevision:7,value:{language:'en',theme:'dark',terminalFontSize:18}});expect(apply).toHaveBeenCalledTimes(2);expect(localStorage.getItem('maulink.terminal.copyOnSelect')).toBe('true');
  });
});
