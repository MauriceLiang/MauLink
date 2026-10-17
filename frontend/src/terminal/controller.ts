import { messages } from "../i18n/locale";
import { terminalMessages } from "../i18n/terminal";
import { shallowRef } from "vue";
import { Channel } from "@tauri-apps/api/core";
import { Terminal, type ITheme } from "@xterm/xterm";
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
import { resolveTerminalAppearance } from "./appearance";
import { resolveTerminalTheme } from "./theme";
import type { ServerAppearance } from "../../../contracts/v1/ServerAppearance";
const t = messages(terminalMessages);
export interface TerminalTab { id: string; connectionId: string; serverId: string; title: string; terminalId: string | null; state: TerminalState; columns: number; rows: number; error: string; inputPaused: boolean; workingDirectory: string | null; }
interface Runtime {
  terminal: Terminal; fit: FitAddon; mount: HTMLElement; channel: Channel<TerminalChunk>;
  selectionHost: HTMLElement | null;
  theme: ITheme; selectionExtent: number;
  opened: TerminalOpenResult | null; early: TerminalChunk[]; earlyBytes: number;
  output: (chunk: TerminalChunk) => Promise<void>; lastOutput: Promise<void>;
  input: Promise<void>; inputSeq: string; inputBytes: number; inputStopped: boolean; outputStopped: boolean;
  closing: boolean; disposed: boolean; timer?: ReturnType<typeof setTimeout>; resizeTimer?: ReturnType<typeof setTimeout>; copyTimer?: ReturnType<typeof setTimeout>;
  resize: Promise<void>; lastSize: string; polling: boolean; ended: boolean;
  directoryGeneration: number; directoryRequests: number;
}

export function createTerminalController(api: ReturnType<typeof createTerminalApi>, channelFactory = () => new Channel<TerminalChunk>()) {
  // Runtime data, Channel chunks and xterm instances never enter Vue reactive state.
  const runtimes = new Map<string, Runtime>();
  const directoryTracking = new Set<string>();
  const tabs = shallowRef<TerminalTab[]>([]);
  let settings = defaultSettings;
  const serverAppearances = new Map<string, ServerAppearance>();
  const systemThemeQuery = typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;
  let systemIsDark = systemThemeQuery?.matches ?? false;
  let copyOnSelect = () => false;
  let number = 1;
  let disposed = false;
  const tab = (id: string) => tabs.value.find(value => value.id === id);
  function update(id: string, patch: Partial<TerminalTab>) {
    const current = tab(id);
    if (!current || Object.entries(patch).every(([key, value]) => current[key as keyof TerminalTab] === value)) return;
    tabs.value = tabs.value.map(value => value.id === id ? { ...value, ...patch } : value);
  }
  function create(connectionId: string, serverId: string) {
    const id = crypto.randomUUID();
    tabs.value = [...tabs.value, { id, connectionId, serverId, title: `Terminal ${number++}`, terminalId: null, state: 'opening', columns: 80, rows: 24, error: '', inputPaused: false, workingDirectory: null }];
    return id;
  }
  function fail(id: string, error: unknown, stream = false) {
    const runtime = runtimes.get(id);
    if (!runtime || runtime.disposed) return;
    runtime.inputStopped = true; runtime.terminal.options.disableStdin = true;
    const mapped = mapError(error);
    update(id, { error: stream && mapped.code === 'INTERNAL' ? t('outputFailed') : presentError(mapped).message, inputPaused: true });
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
        if (!runtime.disposed) runtime.terminal.write(`\r\n\x1b[90m[${t('shellEnded')}]\x1b[0m\r\n`);
        return;
      }
      if (directoryTracking.has(id) && !runtime.directoryRequests) await refreshWorkingDirectory(id);
    } catch (reason) { update(id, { error: presentError(mapError(reason)).message }); }
    finally { runtime.polling = false; }
    if (!runtime.disposed && !runtime.closing) runtime.timer = setTimeout(() => { void poll(id); }, 1200);
  }
  async function refreshWorkingDirectory(id: string): Promise<string | null> {
    const runtime = runtimes.get(id);
    if (!runtime?.opened || runtime.disposed || runtime.closing || tab(id)?.state !== 'running') return null;
    const generation = ++runtime.directoryGeneration;
    ++runtime.directoryRequests;
    try {
      await runtime.input;
      if (runtime.disposed || runtime.closing || generation !== runtime.directoryGeneration) return null;
      let directory: string | null;
      try { directory = await api.workingDirectory({ terminalId: runtime.opened.terminalId }); }
      catch { directory = null; }
      if (runtime.disposed || runtime.closing || generation !== runtime.directoryGeneration) return null;
      update(id, { workingDirectory: directory });
      return directory;
    } finally { --runtime.directoryRequests; }
  }
  function trackWorkingDirectory(id: string, enabled: boolean) {
    if (enabled) directoryTracking.add(id);
    else {
      directoryTracking.delete(id);
      const runtime = runtimes.get(id);
      if (runtime) ++runtime.directoryGeneration;
    }
  }
  function send(id: string, bytes: Uint8Array) {
    const runtime = runtimes.get(id);
    if (!runtime?.opened || runtime.inputStopped || runtime.closing || runtime.disposed || tab(id)?.state !== 'running') return;
    if (runtime.inputBytes + bytes.byteLength > 262144) { update(id, { error: t('inputFull') }); return; }
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
    syncFullSelectionBackground(runtime);
    if (focus) runtime.terminal.focus();
  }
  function clearFullSelectionBackground(host: HTMLElement | null) {
    host?.removeAttribute('data-terminal-full-selection');
    host?.style.removeProperty('--terminal-selection-fill-top');
    host?.style.removeProperty('--terminal-selection-fill-left');
    host?.style.removeProperty('--terminal-selection-fill-width');
    host?.style.removeProperty('--terminal-selection-fill-height');
    host?.style.removeProperty('--terminal-full-selection-background');
    host?.style.removeProperty('--terminal-full-selection-gap-background');
    const mount = host?.querySelector<HTMLElement>('.terminal-mount');
    const selectionLayer = mount?.querySelector<HTMLElement>('.xterm-selection');
    selectionLayer?.style.removeProperty('clip-path');
    selectionLayer?.style.removeProperty('width');
    selectionLayer?.querySelector('.terminal-selection-fill')?.remove();
    const viewport = mount?.querySelector<HTMLElement>('.xterm-viewport');
    viewport?.style.removeProperty('background-image');
    viewport?.style.removeProperty('background-size');
    viewport?.style.removeProperty('background-position');
    viewport?.style.removeProperty('background-repeat');
  }
  function getVisibleContentBounds(runtime: Runtime) {
    const { terminal } = runtime;
    const buffer = terminal.buffer.active;
    let left = terminal.cols;
    let right = -1;
    let top = terminal.rows;
    let bottom = -1;
    for (let row = 0; row < terminal.rows; row++) {
      const line = buffer.getLine(buffer.viewportY + row);
      if (!line) continue;
      for (let column = 0; column < terminal.cols; column++) {
        const cell = line.getCell(column);
        if (!cell || !/\S/u.test(cell.getChars())) continue;
        left = Math.min(left, column);
        right = Math.max(right, column + Math.max(1, cell.getWidth()) - 1);
        top = Math.min(top, row);
        bottom = Math.max(bottom, row);
      }
    }
    const cursorX = Math.max(0, Math.min(terminal.cols - 1, buffer.cursorX));
    const cursorY = Math.max(0, Math.min(terminal.rows - 1, buffer.cursorY));
    left = Math.min(left, cursorX);
    right = Math.max(right, cursorX);
    top = Math.min(top, cursorY);
    bottom = Math.max(bottom, cursorY);
    return { left, right: right + 1, top, bottom: bottom + 1 };
  }
  function syncFullSelectionBackground(runtime: Runtime) {
    const host = runtime.selectionHost;
    if (!host) return;
    const selection = runtime.terminal.getSelectionPosition();
    const buffer = runtime.terminal.buffer.active;
    const isFullSelection = selection?.start.x === 0
      && selection.start.y === 0
      && selection.end.x === runtime.terminal.cols
      && selection.end.y === buffer.baseY + runtime.terminal.rows - 1;
    if (!isFullSelection) {
      clearFullSelectionBackground(host);
      return;
    }
    const theme = runtime.theme;
    const selectionBackground = theme?.selectionBackground ?? 'rgba(59, 130, 246, 0.28)';
    host.style.setProperty('--terminal-full-selection-background', selectionBackground);
    const composedBackground = compositeSelectionBackground(selectionBackground, theme?.background ?? '#000000');
    host.style.setProperty('--terminal-full-selection-gap-background', composedBackground);

    const screen = runtime.mount.querySelector<HTMLElement>('.xterm-screen');
    const viewport = runtime.mount.querySelector<HTMLElement>('.xterm-viewport');
    const selectionLayer = runtime.mount.querySelector<HTMLElement>('.xterm-selection');
    const hostRect = host.getBoundingClientRect();
    const screenRect = screen?.getBoundingClientRect();
    const viewportRect = viewport?.getBoundingClientRect();
    if (!hostRect.width || !hostRect.height || !screenRect?.width || !screenRect.height || !viewportRect?.width || !viewportRect.height || !runtime.terminal.cols || !runtime.terminal.rows) return;

    const content = getVisibleContentBounds(runtime);
    const cellWidth = screenRect.width / runtime.terminal.cols;
    const cellHeight = screenRect.height / runtime.terminal.rows;
    const amount = Math.max(0, Math.min(100, runtime.selectionExtent)) / 100;
    const contentLeft = screenRect.left + content.left * cellWidth;
    const contentRight = screenRect.left + content.right * cellWidth;
    const contentTop = screenRect.top + content.top * cellHeight;
    const contentBottom = screenRect.top + content.bottom * cellHeight;
    const target = {
      left: contentLeft + (hostRect.left - contentLeft) * amount,
      top: contentTop + (hostRect.top - contentTop) * amount,
      right: contentRight + (hostRect.right - contentRight) * amount,
      bottom: contentBottom + (hostRect.bottom - contentBottom) * amount,
    };
    host.style.setProperty('--terminal-selection-fill-left', `${target.left - hostRect.left}px`);
    host.style.setProperty('--terminal-selection-fill-top', `${target.top - hostRect.top}px`);
    host.style.setProperty('--terminal-selection-fill-width', `${target.right - target.left}px`);
    host.style.setProperty('--terminal-selection-fill-height', `${target.bottom - target.top}px`);

    if (selectionLayer) {
      selectionLayer.style.width = `${screenRect.width}px`;
      const inset = {
        top: Math.max(0, target.top - screenRect.top),
        right: Math.max(0, screenRect.right - target.right),
        bottom: Math.max(0, screenRect.bottom - target.bottom),
        left: Math.max(0, target.left - screenRect.left),
      };
      selectionLayer.style.clipPath = `inset(${inset.top}px ${inset.right}px ${inset.bottom}px ${inset.left}px)`;
      let fill = selectionLayer.querySelector<HTMLElement>('.terminal-selection-fill');
      if (!fill) {
        fill = document.createElement('div');
        fill.className = 'terminal-selection-fill';
        fill.setAttribute('aria-hidden', 'true');
        selectionLayer.append(fill);
      }
      const selectedArea = {
        left: Math.max(target.left, screenRect.left),
        top: Math.max(target.top, screenRect.top),
        right: Math.min(target.right, screenRect.right),
        bottom: Math.min(target.bottom, screenRect.bottom),
      };
      fill.style.left = `${selectedArea.left - screenRect.left}px`;
      fill.style.top = `${selectedArea.top - screenRect.top}px`;
      fill.style.width = `${Math.max(0, selectedArea.right - selectedArea.left)}px`;
      fill.style.height = `${Math.max(0, selectedArea.bottom - selectedArea.top)}px`;
      fill.style.backgroundColor = composedBackground;
    }
    if (viewport) {
      viewport.style.backgroundImage = `linear-gradient(${composedBackground}, ${composedBackground})`;
      viewport.style.backgroundSize = `${target.right - target.left}px ${target.bottom - target.top}px`;
      viewport.style.backgroundPosition = `${target.left - viewportRect.left}px ${target.top - viewportRect.top}px`;
      viewport.style.backgroundRepeat = 'no-repeat';
    }
    host.setAttribute('data-terminal-full-selection', 'true');
  }
  function compositeSelectionBackground(selection: string, background: string) {
    const selected = parseCssColor(selection);
    const base = parseCssColor(background);
    if (!selected || !base) return selection;
    const alpha = selected.a + base.a * (1 - selected.a);
    if (alpha === 0) return 'rgba(0, 0, 0, 0)';
    const channel = (front: number, behind: number) => Math.round((front * selected.a + behind * base.a * (1 - selected.a)) / alpha);
    return `rgba(${channel(selected.r, base.r)}, ${channel(selected.g, base.g)}, ${channel(selected.b, base.b)}, ${alpha})`;
  }
  function parseCssColor(value: string) {
    const hex = /^#([\da-f]{6})([\da-f]{2})?$/i.exec(value);
    if (hex) {
      const color = hex[1]!;
      return {
        r: Number.parseInt(color.slice(0, 2), 16),
        g: Number.parseInt(color.slice(2, 4), 16),
        b: Number.parseInt(color.slice(4, 6), 16),
        a: hex[2] ? Number.parseInt(hex[2], 16) / 255 : 1,
      };
    }
    const rgb = /^rgba?\(\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)(?:\s*,\s*(\d+(?:\.\d+)?))?\s*\)$/i.exec(value);
    if (!rgb) return null;
    return { r: Number(rgb[1]), g: Number(rgb[2]), b: Number(rgb[3]), a: rgb[4] === undefined ? 1 : Number(rgb[4]) };
  }
  function applyTerminalTheme() {
    const globalTheme = resolveTerminalTheme(settings, settings.theme, systemIsDark).theme;
    const root = document.documentElement.style;
    root.setProperty('--color-terminal-bg', globalTheme.background);
    root.setProperty('--color-terminal-text', globalTheme.foreground);
    root.setProperty('--color-terminal-cursor', globalTheme.cursor);
    root.setProperty('--color-terminal-selection', globalTheme.selectionBackground);
    runtimes.forEach((runtime, id) => {
      const metadata = tab(id);
      if (!metadata) return;
      const effective = resolveTerminalAppearance(metadata.serverId, settings, serverAppearances.get(metadata.serverId));
      const appearance = resolveTerminalTheme(effective, effective.theme, systemIsDark);
      const theme = appearance.theme;
      runtime.theme = theme;
      runtime.selectionExtent = effective.terminalSelectionExtent;
      runtime.terminal.options.theme = theme;
      syncFullSelectionBackground(runtime);
    });
  }
  const onSystemThemeChange = (event: MediaQueryListEvent) => {
    systemIsDark = event.matches;
    if (settings.theme === 'system') applyTerminalTheme();
  };
  systemThemeQuery?.addEventListener('change', onSystemThemeChange);
  function release(id: string) {
    const runtime = runtimes.get(id); if (!runtime) return;
    runtime.disposed = true;
    clearFullSelectionBackground(runtime.selectionHost);
    runtime.selectionHost = null;
    directoryTracking.delete(id);
    clearTimeout(runtime.timer); clearTimeout(runtime.resizeTimer); clearTimeout(runtime.copyTimer);
    runtime.channel.onmessage = () => undefined;
    runtime.terminal.dispose(); runtime.mount.remove(); runtimes.delete(id);
  }
  async function attach(id: string, host: HTMLElement) {
    let runtime = runtimes.get(id);
    if (runtime) {
      if (runtime.selectionHost !== host) clearFullSelectionBackground(runtime.selectionHost);
      host.append(runtime.mount);
      runtime.selectionHost = host;
      fit(id);
      syncFullSelectionBackground(runtime);
      return;
    }
    const metadata = tab(id); if (!metadata || disposed) return;
    const mount = document.createElement('div'); mount.className = 'terminal-mount'; host.append(mount);
    const effective = resolveTerminalAppearance(metadata.serverId, settings, serverAppearances.get(metadata.serverId));
    const appearance = resolveTerminalTheme(effective, effective.theme, systemIsDark);
    const terminal = new Terminal({ allowProposedApi: false, allowTransparency: appearance.transparent, cursorBlink: settings.terminalCursorBlink, screenReaderMode: true, fontFamily: settings.terminalFontFamily, fontSize: settings.terminalFontSize, cursorStyle: settings.terminalCursorStyle, scrollback: settings.terminalScrollbackLines, lineHeight: settings.terminalLineHeight, theme: appearance.theme });
    const addon = new FitAddon(); terminal.loadAddon(addon); terminal.open(mount);
    const channel = channelFactory();
    runtime = { terminal, fit: addon, mount, channel, selectionHost: host, theme: appearance.theme, selectionExtent: effective.terminalSelectionExtent, opened: null, early: [], earlyBytes: 0, output: () => Promise.resolve(), lastOutput: Promise.resolve(), input: Promise.resolve(), inputSeq: '1', inputBytes: 0, inputStopped: false, outputStopped: false, closing: false, disposed: false, resize: Promise.resolve(), lastSize: '', polling: false, ended: false, directoryGeneration: 0, directoryRequests: 0 };
    const entry = runtime;
    runtimes.set(id, entry);
    entry.output = createOutputConsumer((bytes, done) => terminal.write(bytes, done), chunk => api.ack({ terminalId: chunk.terminalId, streamId: chunk.streamId, seq: chunk.seq }));
    channel.onmessage = chunk => receive(id, chunk);
    terminal.onData(text => send(id, new TextEncoder().encode(text)));
    terminal.onBinary(text => send(id, Uint8Array.from(text, character => character.charCodeAt(0))));
    terminal.onResize(({ cols, rows }) => { update(id, { columns: cols, rows }); syncFullSelectionBackground(entry); resize(id); });
    terminal.onSelectionChange(() => {
      syncFullSelectionBackground(entry);
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
      if (entry.disposed || disposed) {
        try { await api.close({ terminalId: opened.terminalId }); } catch { /* The SSH connection may already have ended. */ }
        return;
      }
      update(id, { terminalId: opened.terminalId, state: 'running' });
      if (entry.outputStopped) { const closed = await api.close({ terminalId: opened.terminalId }); update(id, { state: closed.state }); return; }
      for (const chunk of entry.early.splice(0)) receive(id, chunk);
      entry.earlyBytes = 0;
      fit(id, true); entry.timer = setTimeout(() => { void poll(id); }, 1200);
    } catch (reason) { if (!entry.disposed) { update(id, { state: 'failed', error: presentError(mapError(reason)).message }); terminal.options.disableStdin = true; } }
  }
  function detach(id: string) {
    const runtime = runtimes.get(id);
    if (!runtime) return;
    clearFullSelectionBackground(runtime.selectionHost);
    runtime.selectionHost = null;
    runtime.mount.remove();
  }
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
  function disposeConnection(connectionId: string) {
    const ids = tabs.value.filter(value => value.connectionId === connectionId).map(value => value.id);
    ids.forEach(release);
    if (ids.length) tabs.value = tabs.value.filter(value => value.connectionId !== connectionId);
  }
  function applySettings(value: AppSettings) {
    settings = value;
    applyTerminalTheme();
    runtimes.forEach((runtime, id) => { runtime.terminal.options.fontFamily = value.terminalFontFamily; runtime.terminal.options.fontSize = value.terminalFontSize; runtime.terminal.options.cursorStyle = value.terminalCursorStyle; runtime.terminal.options.scrollback = value.terminalScrollbackLines; runtime.terminal.options.lineHeight = value.terminalLineHeight; runtime.terminal.options.cursorBlink = value.terminalCursorBlink; fit(id); });
  }
  function setServerAppearance(serverId: string, appearance: ServerAppearance | undefined) {
    if (appearance) serverAppearances.set(serverId, appearance);
    else serverAppearances.delete(serverId);
    applyTerminalTheme();
  }
  function setCopyPreference(read: () => boolean) { copyOnSelect = read; }
  function focus(id: string) {
    const runtime = runtimes.get(id);
    fit(id, true);
    if (runtime && !runtime.disposed) runtime.terminal.refresh(0, runtime.terminal.rows - 1);
  }
  function clear(id: string) { runtimes.get(id)?.terminal.clear(); focus(id); }
  function dispose() { disposed = true; systemThemeQuery?.removeEventListener('change', onSystemThemeChange); [...runtimes.keys()].forEach(release); ['--color-terminal-bg', '--color-terminal-text', '--color-terminal-cursor', '--color-terminal-selection'].forEach(name => document.documentElement.style.removeProperty(name)); }
  return { tabs, create, attach, detach, fit, focus, clear, close, refreshConnection, disposeConnection, refreshWorkingDirectory, trackWorkingDirectory, applySettings, setServerAppearance, setCopyPreference, dispose };
}
export type TerminalController = ReturnType<typeof createTerminalController>;
export type TerminalChannelFactory = () => Channel<TerminalChunk>;
