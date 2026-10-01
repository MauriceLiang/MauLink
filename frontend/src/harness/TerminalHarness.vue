<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import type { Channel } from "@tauri-apps/api/core";
import type { TerminalChunk } from "../../../contracts/v1/TerminalChunk";
import type { TerminalOpenResult } from "../../../contracts/v1/TerminalOpenResult";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import AppShell from "../app/AppShell.vue";
import { createIpcClient, type IpcTransport } from "../ipc/client";
import { createMockIpc } from "../ipc/mock";
import { shellAppInfo } from "./shell-fixtures";
// Development-only bridge to the existing Core and isolated loopback OpenSSH fixture.
// Production still uses Tauri Channel; neither this transport nor the bridge is bundled.
const endpoint = 'http://127.0.0.1:1421';
const client = ref<ReturnType<typeof createIpcClient> | null>(null);
const error = ref('');
const interrupts = ref(0);
const keyboard = ref('');
function observeKey(event: KeyboardEvent) {
  if (event.ctrlKey && event.key.toLowerCase() === 'c') keyboard.value = `key=${event.key}, keyCode=${event.keyCode}, ctrl=${event.ctrlKey}`;
}
const streams = new Map<string, AbortController>();
const channelFactory = () => ({ onmessage: (_chunk: TerminalChunk) => {} } as Channel<TerminalChunk>);
async function json(url: string, options?: RequestInit) {
  const response = await fetch(endpoint + url, options);
  const value: unknown = await response.json();
  if (!response.ok) throw value;
  return value;
}
async function output(opened: TerminalOpenResult, channel: Channel<TerminalChunk>, abort: AbortController) {
  try {
    while (!abort.signal.aborted) {
      const value = await json('/output/' + opened.terminalId, { signal: abort.signal }) as TerminalChunk | null;
      if (value) channel.onmessage(value);
    }
  } catch { if (!abort.signal.aborted) error.value = '验收桥接输出中断。'; }
}
onMounted(async () => {
  document.addEventListener("keydown", observeKey, true);
  document.documentElement.dataset.theme = new URLSearchParams(location.search).get('theme') === 'dark' ? 'dark' : 'light';
  try {
    const { server } = await json('/bootstrap') as { server: ServerProfile };
    const metadata = createMockIpc({ app_get_info: () => shellAppInfo, server_list: () => ({ items: [server], nextCursor: null }), server_get: () => server, group_list: () => [] });
    const transport: IpcTransport = { async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      if (['app_get_info', 'server_list', 'server_get', 'group_list'].includes(command)) return metadata.invoke<T>(command, args);
      const result = await json('/ipc/' + command, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(args?.request) });
      if (command === 'terminal_write' && (args?.request as { payload: { dataBase64: string } }).payload.dataBase64 === 'Aw==') interrupts.value++;
      if (command === 'terminal_open') {
        const opened = result as TerminalOpenResult; const abort = new AbortController(); streams.set(opened.terminalId, abort);
        void output(opened, args?.outputChannel as Channel<TerminalChunk>, abort);
      }
      if (command === 'terminal_close') { const id = (args?.request as { payload: { terminalId: string } }).payload.terminalId; streams.get(id)?.abort(); streams.delete(id); }
      return result as T;
    } };
    client.value = createIpcClient(transport);
  } catch { error.value = '请先启动 Phase 6 回环 Core 验收桥接。'; }
});
onBeforeUnmount(() => { document.removeEventListener("keydown", observeKey, true); streams.forEach(abort => abort.abort()); });
</script>
<template>
  <AppShell v-if="client" :client="client" :terminal-channel-factory="channelFactory" />
  <output v-if="client" class="terminal-harness-counter" aria-label="Core 接受的 Ctrl+C 次数">Ctrl+C ACK: {{ interrupts }} · {{ keyboard }}</output>
  <p v-if="error" role="alert">{{ error }}</p>
</template>

<style scoped>
.terminal-harness-counter { position: fixed; bottom: 8px; left: 50%; z-index: 50; font-size: 11px; color: var(--color-text-secondary); }
</style>
