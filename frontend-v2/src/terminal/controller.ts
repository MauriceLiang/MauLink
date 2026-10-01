import { shallowRef } from "vue";
import { Channel } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import type { AppSettings } from "../../../contracts/v1/AppSettings";
import type { TerminalChunk } from "../../../contracts/v1/TerminalChunk";
import type { TerminalOpenResult } from "../../../contracts/v1/TerminalOpenResult";
import type { TerminalState } from "../../../contracts/v1/TerminalState";
import type { createTerminalApi } from "../ipc/terminal";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { createOutputConsumer, encodeBytesBase64, nextSequence } from "./codec";
import { defaultSettings } from "./preferences";
export interface TerminalTab { id: string; connectionId: string; title: string; terminalId: string | null; state: TerminalState; columns: number; rows: number; error: string; inputPaused: boolean; }
interface Runtime {
  terminal: Terminal; fit: FitAddon; mount: HTMLElement; channel: Channel<TerminalChunk>;
  opened: TerminalOpenResult | null; early: TerminalChunk[]; earlyBytes: number;
  output: (chunk: TerminalChunk) => Promise<void>; lastOutput: Promise<void>;
  input: Promise<void>; inputSeq: string; inputBytes: number; inputStopped: boolean; outputStopped: boolean;
  closing: boolean; disposed: boolean; timer?: ReturnType<typeof setTimeout>; resizeTimer?: ReturnType<typeof setTimeout>; copyTimer?: ReturnType<typeof setTimeout>;
  resize: Promise<void>; lastSize: string; polling: boolean; ended: boolean;
}

export function createTerminalController(api: ReturnType<typeof createTerminalApi>, channelFactory = () => new Channel<TerminalChunk>()) {
  // Runtime data, Channel chunks and xterm instances never enter Vue reactive state.
  const runtimes = new Map<string, Runtime>();
  const tabs = shallowRef<TerminalTab[]>([]);
  let settings = defaultSettings;
  let copyOnSelect = () => false;
  let number = 1;
  let disposed = false;
  const tab = (id: string) => tabs.value.find(value => value.id === id);
  function update(id: string, patch: Partial<TerminalTab>) {
    const current = tab(id);
    if (!current || Object.entries(patch).every(([key, value]) => current[key as keyof TerminalTab] === value)) return;
    tabs.value = tabs.value.map(value => value.id === id ? { ...value, ...patch } : value);
  }
  function create(connectionId: string) {
    const id = crypto.randomUUID();
    tabs.value = [...tabs.value, { id, connectionId, title: `终端 ${number++}`, terminalId: null, state: 'opening', columns: 80, rows: 24, error: '', inputPaused: false }];
    return id;
  }
  function fail(id: string, error: unknown, stream = false) {
    const runtime = runtimes.get(id);
    if (!runtime || runtime.disposed) return;
    runtime.inputStopped = true; runtime.terminal.options.disableStdin = true;
    const mapped = mapError(error);
    update(id, { error: stream && mapped.code === 'INTERNAL' ? '终端输出校验或确认失败，已暂停输入。请关闭此终端后重新打开。' : presentError(mapped).message, inputPaused: true });
    if (stream && !runtime.outputStopped) {
      runtime.outputStopped = true;
      if (runtime.opened) void api.close({ terminalId: runtime.opened.terminalId }).then(snapshot => update(id, { state: snapshot.state })).catch(reason => update(id, { error: presentError(mapError(reason)).message }));
    }
  }
  function receive(id: string, chunk: TerminalChunk) {
    const runtime = runtimes.get(id);
    if (!runtime || runtime.disposed || runtime.outputStopped) return;
    if (!runtime.opened) {
      if (runtime.earlyBytes + chunk.byteLength > 131072 || runtime.early.length >= 4) { fail(id, new Error('终端预启动流控失败'), true); return; }
      runtime.early.push(chunk); runtime.earlyBytes += chunk.byteLength; return;
    }
    if (chunk.terminalId !== runtime.opened.terminalId || chunk.streamId !== runtime.opened.streamId) { fail(id, new Error('无效终端流'), true); return; }
    runtime.lastOutput = runtime.output(chunk);
    void runtime.lastOutput.catch(error => fail(id, error, true));
  }
  async function poll(id: string) {
    const runtime = runtimes.get(id);
    if (!runtime?.opened || runtime.disposed || runtime.closing || runtime.polling || runtime.ended) return;
    clearTimeout(runtime.timer); runtime.polling = true;
    try {
      const snapshot = await api.get({ terminalId: runtime.opened.terminalId });
      if (runtime.disposed || runtime.closing) return;
      update(id, { state: snapshot.state });
      if (snapshot.error) update(id, { error: presentError(snapshot.error).message });
      if (snapshot.state === 'closed' || snapshot.state === 'failed') {
        runtime.ended = true; runtime.inputStopped = true; runtime.terminal.options.disableStdin = true;
        await runtime.lastOutput.catch(() => undefined);
        if (!runtime.disposed) runtime.terminal.write('\r\n\x1b[90m[远程 Shell 已结束]\x1b[0m\r\n');
        return;
      }
    } catch (reason) { update(id, { error: presentError(mapError(reason)).message }); }
    finally { runtime.polling = false; }
    if (!runtime.disposed && !runtime.closing) runtime.timer = setTimeout(() => { void poll(id); }, 1200);
  }
  function send(id: string, bytes: Uint8Array) {
    const runtime = runtimes.get(id);
    if (!runtime?.opened || runtime.inputStopped || runtime.closing || runtime.disposed || tab(id)?.state !== 'running') return;
    if (runtime.inputBytes + bytes.byteLength > 262144) { update(id, { error: '输入暂存已满，请等待发送完成后再输入。' }); return; }
    runtime.inputBytes += bytes.byteLength;
    runtime.input = runtime.input.then(async () => {
      for (let offset = 0; offset < bytes.length; offset += 65536) {
        if (runtime.inputStopped || runtime.disposed || runtime.closing) return;
        const result = await api.write({ terminalId: runtime.opened!.terminalId, inputSeq: String(runtime.inputSeq), dataBase64: encodeBytesBase64(bytes.subarray(offset, offset + 65536)) });
        if (result.inputSeq !== String(runtime.inputSeq)) throw new Error('无效输入确认');
        runtime.inputSeq = nextSequence(runtime.inputSeq);
      }
    }).catch(error => fail(id, error)).finally(() => { runtime.inputBytes -= bytes.byteLength; });
  }
  function resize(id: string) {
    const runtime = runtimes.get(id);
    if (!runtime?.opened || runtime.closing || runtime.disposed || tab(id)?.state !== 'running') return;
    clearTimeout(runtime.resizeTimer);
    runtime.resizeTimer = setTimeout(() => {
      const rect = runtime.mount.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      const size = { columns: runtime.terminal.cols, rows: runtime.terminal.rows, pixelWidth: Math.round(rect.width), pixelHeight: Math.round(rect.height) };
      const signature = JSON.stringify(size);
      if (signature === runtime.lastSize) return;
      runtime.resize = runtime.resize.then(async () => {
        if (runtime.disposed || runtime.closing) return;
        const result = await api.resize({ terminalId: runtime.opened!.terminalId, ...size });
        runtime.lastSize = signature; update(id, { columns: result.columns, rows: result.rows });
      }).catch(reason => update(id, { error: presentError(mapError(reason)).message }));
    }, 80);
  }
  function fit(id: string, focus = false) {
    const runtime = runtimes.get(id);
    if (!runtime || runtime.disposed || !runtime.mount.isConnected || !runtime.mount.getBoundingClientRect().width || !runtime.mount.getBoundingClientRect().height) return;
    runtime.fit.fit(); resize(id);
    if (focus) runtime.terminal.focus();
  }
  function release(id: string) {
    const runtime = runtimes.get(id); if (!runtime) return;
    runtime.disposed = true;
    clearTimeout(runtime.timer); clearTimeout(runtime.resizeTimer); clearTimeout(runtime.copyTimer);
    runtime.channel.onmessage = () => undefined;
    runtime.terminal.dispose(); runtime.mount.remove(); runtimes.delete(id);
  }
  async function attach(id: string, host: HTMLElement) {
    let runtime = runtimes.get(id);
    if (runtime) { host.append(runtime.mount); fit(id); return; }
    const metadata = tab(id); if (!metadata || disposed) return;
    const mount = document.createElement('div'); mount.className = 'terminal-mount'; host.append(mount);
    const terminal = new Terminal({ allowProposedApi: false, cursorBlink: true, screenReaderMode: true, fontFamily: settings.terminalFontFamily, fontSize: settings.terminalFontSize, cursorStyle: settings.terminalCursorStyle, scrollback: settings.terminalScrollbackLines, lineHeight: 1.35, theme: { background: '#000000', foreground: '#a7b0be', cursor: '#5b5ce2', selectionBackground: '#373785' } });
    const addon = new FitAddon(); terminal.loadAddon(addon); terminal.open(mount);
    const channel = channelFactory();
    runtime = { terminal, fit: addon, mount, channel, opened: null, early: [], earlyBytes: 0, output: () => Promise.resolve(), lastOutput: Promise.resolve(), input: Promise.resolve(), inputSeq: '1', inputBytes: 0, inputStopped: false, outputStopped: false, closing: false, disposed: false, resize: Promise.resolve(), lastSize: '', polling: false, ended: false };
    const entry = runtime;
    runtimes.set(id, entry);
    entry.output = createOutputConsumer((bytes, done) => terminal.write(bytes, done), chunk => api.ack({ terminalId: chunk.terminalId, streamId: chunk.streamId, seq: chunk.seq }));
    channel.onmessage = chunk => receive(id, chunk);
    terminal.onData(text => send(id, new TextEncoder().encode(text)));
    terminal.onBinary(text => send(id, Uint8Array.from(text, character => character.charCodeAt(0))));
    terminal.onResize(({ cols, rows }) => { update(id, { columns: cols, rows }); resize(id); });
    terminal.onSelectionChange(() => {
      clearTimeout(entry.copyTimer);
      if (!copyOnSelect()) return;
      entry.copyTimer = setTimeout(() => {
        if (!copyOnSelect() || entry.disposed) return;
        const selection = terminal.getSelection();
        if (selection) void navigator.clipboard.writeText(selection).catch(reason => update(id, { error: presentError(mapError(reason)).message }));
      }, 120);
    });
    try {
      fit(id);
      const rect = mount.getBoundingClientRect();
      const opened = await api.open({ connectionId: metadata.connectionId, columns: terminal.cols, rows: terminal.rows, pixelWidth: Math.round(rect.width) || null, pixelHeight: Math.round(rect.height) || null }, channel);
      entry.opened = opened;
      if (entry.disposed || disposed) return;
      update(id, { terminalId: opened.terminalId, state: 'running' });
      if (entry.outputStopped) { const closed = await api.close({ terminalId: opened.terminalId }); update(id, { state: closed.state }); return; }
      for (const chunk of entry.early.splice(0)) receive(id, chunk);
      entry.earlyBytes = 0;
      fit(id, true); entry.timer = setTimeout(() => { void poll(id); }, 1200);
    } catch (reason) { if (!entry.disposed) { update(id, { state: 'failed', error: presentError(mapError(reason)).message }); terminal.options.disableStdin = true; } }
  }
  function detach(id: string) { runtimes.get(id)?.mount.remove(); }
  async function close(id: string) {
    const entry = runtimes.get(id); const metadata = tab(id);
    if (!entry || !metadata || entry.closing || metadata.state === 'opening') return false;
    entry.closing = true; update(id, { state: 'closing' });
    clearTimeout(entry.timer); clearTimeout(entry.resizeTimer);
    try {
      if (entry.opened) await api.close({ terminalId: entry.opened.terminalId });
      release(id); tabs.value = tabs.value.filter(value => value.id !== id); return true;
    } catch (reason) {
      entry.closing = false; update(id, { state: metadata.state, error: presentError(mapError(reason)).message });
      entry.timer = setTimeout(() => { void poll(id); }, 1200); return false;
    }
  }
  async function refreshConnection(connectionId: string) { await Promise.all(tabs.value.filter(value => value.connectionId === connectionId).map(value => poll(value.id))); }
  function applySettings(value: AppSettings) {
    settings = value;
    runtimes.forEach((runtime, id) => { runtime.terminal.options.fontFamily = value.terminalFontFamily; runtime.terminal.options.fontSize = value.terminalFontSize; runtime.terminal.options.cursorStyle = value.terminalCursorStyle; runtime.terminal.options.scrollback = value.terminalScrollbackLines; fit(id); });
  }
  function setCopyPreference(read: () => boolean) { copyOnSelect = read; }
  function focus(id: string) { fit(id, true); }
  function clear(id: string) { runtimes.get(id)?.terminal.clear(); focus(id); }
  function dispose() { disposed = true; [...runtimes.keys()].forEach(release); }
  return { tabs, create, attach, detach, fit, focus, clear, close, refreshConnection, applySettings, setCopyPreference, dispose };
}
export type TerminalController = ReturnType<typeof createTerminalController>;
export type TerminalChannelFactory = () => Channel<TerminalChunk>;
