<script setup lang="ts">
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../../contracts/v1/TerminalChunk';
import type { SftpTransferSnapshot } from '../../../contracts/v1/SftpTransferSnapshot';
import AppShell from '../app/AppShell.vue';
import { createIpcClient } from '../ipc/client';
import { visualConfig, createVisualMock, createVisualStorage } from './visual-fixtures';
const config = visualConfig(new URLSearchParams(location.search));
// Shadow storage only in this DEV document; reload restores the browser's native storage object.
Object.defineProperty(window, 'localStorage', {value:createVisualStorage(), configurable:true});
const client = createIpcClient(createVisualMock(config));
const terminalChannelFactory = () => ({ onmessage: (_value: TerminalChunk) => undefined }) as Channel<TerminalChunk>;
const transferChannelFactory = () => ({ onmessage: (_value: SftpTransferSnapshot) => undefined }) as Channel<SftpTransferSnapshot>;
document.title = 'MauLink · DEV deterministic Mock IPC';
document.documentElement.dataset.visualFixture = config.page;
</script>
<template><AppShell :client="client" :terminal-channel-factory="terminalChannelFactory" :transfer-channel-factory="transferChannelFactory" /></template>
<style>
/* Only imported by the DEV visual route. Keep focus rings; suppress time-dependent raster changes. */
html[data-visual-fixture] *, html[data-visual-fixture] *::before, html[data-visual-fixture] *::after { animation: none !important; transition: none !important; caret-color: transparent !important; }
html[data-visual-fixture] .xterm-cursor { visibility: hidden !important; }
</style>
