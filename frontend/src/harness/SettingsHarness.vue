<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../../contracts/v1/TerminalChunk';
import type { SftpTransferSnapshot } from '../../../contracts/v1/SftpTransferSnapshot';
import AppShell from '../app/AppShell.vue';
import { createIpcClient, type IpcTransport } from '../ipc/client';
import { createFilesMock, createFilesWorkspaceMock } from './files-fixtures';
import { createSettingsMock } from './settings-fixtures';
import { createCredentialRevealMock } from './credential-reveal-fixtures';
const query = new URLSearchParams(location.search);
const readFailure = ref(false); const conflict = ref(false); const revision = ref(1); const fontSize = ref(14);
const files = createFilesMock(); const workspace = createFilesWorkspaceMock(files);
const settings = createSettingsMock({ readFailure: () => readFailure.value, conflict: () => conflict.value, changed: value => { revision.value = value.revision; fontSize.value = value.value.terminalFontSize; } });
const credentialReveal = createCredentialRevealMock({ nativeAuthAvailable: query.get('nativeAuth') !== '0' });
const credentialRevealCommands = new Set(['reveal_policy_get', 'reveal_policy_enable_protected', 'reveal_policy_enable_direct', 'reveal_policy_set_deny', 'reveal_policy_change_password', 'reveal_policy_recover', 'credential_reveal']);
const transport: IpcTransport = { invoke: <T,>(command: string, args?: Record<string, unknown>) => command.startsWith('settings_') ? settings.transport.invoke<T>(command, args) : credentialRevealCommands.has(command) ? credentialReveal.transport.invoke<T>(command, args) : workspace.invoke<T>(command, args) };
const client = createIpcClient(transport);
const terminalChannelFactory = () => ({ onmessage: (_value: TerminalChunk) => undefined }) as Channel<TerminalChunk>;
const transferChannelFactory = () => ({ onmessage: (_value: SftpTransferSnapshot) => undefined }) as Channel<SftpTransferSnapshot>;
onBeforeUnmount(files.dispose);
</script>
<template><AppShell :client="client" :terminal-channel-factory="terminalChannelFactory" :transfer-channel-factory="transferChannelFactory" /><details class="settings-harness"><summary>Phase 9 · DEV memory Settings fixture</summary><label><input v-model="readFailure" type="checkbox" />Settings read failure</label><label><input v-model="conflict" type="checkbox" />Settings revision conflict</label><output>revision={{ revision }} · fontSize={{ fontSize }}</output><p>Mock settings and identity checks. No profile or credentials written.</p></details></template>
<style scoped>.settings-harness { position: fixed; bottom: 32px; right: 12px; background: var(--color-surface); color: var(--color-text-secondary); border: 1px solid var(--color-border); border-radius: 8px; padding: 8px; z-index: 4; max-width: 250px; font-size: 11px; }.settings-harness label { display: block; margin: 8px 0; }</style>
