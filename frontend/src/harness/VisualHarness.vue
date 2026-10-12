<script setup lang="ts">
import type { Channel } from '@tauri-apps/api/core';
import type { TerminalChunk } from '../../../contracts/v1/TerminalChunk';
import type { SftpTransferSnapshot } from '../../../contracts/v1/SftpTransferSnapshot';
import AppShell from '../app/AppShell.vue';
import { createIpcClient } from '../ipc/client';
import { visualConfig, createVisualMock, createVisualStorage } from './visual-fixtures';
import { createMockUpdaterAdapter, type MockUpdaterScenario } from '../updater/updater-adapter';
const config = visualConfig(new URLSearchParams(location.search));
const updateParam = new URLSearchParams(location.search).get('update');
const updateScenario: MockUpdaterScenario = ['available', 'stable-to-preview-upgrade', 'downgrade', 'channel-empty', 'network-error', 'invalid-index', 'signature-error', 'unsupported'].includes(updateParam ?? '') ? updateParam as MockUpdaterScenario : 'up-to-date';
const updaterAdapter = createMockUpdaterAdapter(updateScenario).adapter;
const visualAppVersion = updateScenario === 'stable-to-preview-upgrade' ? '0.1.1' : updateScenario === 'downgrade' ? '0.1.2' : '0.1.1-rc.8';
const visualAssetUrl = (path: string) => path;
// Shadow storage only in this DEV document; reload restores the browser's native storage object.
Object.defineProperty(window, 'localStorage', {value:createVisualStorage(), configurable:true});
const client = createIpcClient(createVisualMock(config, visualAppVersion));
const terminalChannelFactory = () => ({ onmessage: (_value: TerminalChunk) => undefined }) as Channel<TerminalChunk>;
const transferChannelFactory = () => ({ onmessage: (_value: SftpTransferSnapshot) => undefined }) as Channel<SftpTransferSnapshot>;
document.title = 'MauLink · DEV deterministic Mock IPC';
document.documentElement.dataset.visualFixture = config.page;
</script>
<template><AppShell :client="client" :terminal-channel-factory="terminalChannelFactory" :transfer-channel-factory="transferChannelFactory" :background-image-asset-url="visualAssetUrl" :updater-adapter="updaterAdapter" /></template>
<style>
/* Only imported by the DEV visual route. Keep focus rings; suppress time-dependent raster changes. */
html[data-visual-fixture] *, html[data-visual-fixture] *::before, html[data-visual-fixture] *::after { animation: none !important; transition: none !important; caret-color: transparent !important; }
/* Pin native overlay scrollbars in DEV captures; their OS fade is unrelated to app state. */
html[data-visual-fixture] .files-table-scroll::-webkit-scrollbar { width: 8px; height: 8px; }
html[data-visual-fixture] .files-table-scroll::-webkit-scrollbar-track { background: var(--color-bg-subtle); }
html[data-visual-fixture] .files-table-scroll::-webkit-scrollbar-thumb { background: var(--color-border-hover); border-radius: 4px; }
html[data-visual-fixture] .xterm-cursor { visibility: hidden !important; }
</style>
