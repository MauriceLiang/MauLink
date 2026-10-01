<script setup lang="ts">
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ShellIcon from "./ShellIcon.vue";
const t = messages(shellMessages);
defineProps<{ query: string; home: boolean; shortcut: string; canManage: boolean; settingsEnabled?: boolean }>();
defineEmits<{ "update:query": [value: string]; home: []; about: []; add: []; settings: []; palette: [] }>();
</script>

<template>
  <header class="shell-topbar" data-tauri-drag-region>
    <div class="shell-brand-group" data-tauri-drag-region>
      <button class="shell-brand" type="button" :aria-label="t('maulinkServerWorkspace')" @click="$emit('home')">
        <span class="shell-brand-mark" aria-hidden="true">M</span><span>MauLink</span>
      </button>
      <BaseIconButton class="shell-home-button" :label="t('servers')" :aria-current="home ? 'page' : undefined" @click="$emit('home')"><ShellIcon name="house" /></BaseIconButton>
    </div>
    <div class="shell-search shell-global-search">
      <ShellIcon name="search" />
      <BaseInput id="shell-global-search" :label="t('searchServersOrHosts')" type="search" autocomplete="off"
        :model-value="query" :placeholder="t('searchServersOrHosts2')" @update:model-value="$emit('update:query', $event)" />
      <button type="button" class="palette-trigger" :aria-label="t('commandPalette')" @click="$emit('palette')"><kbd>{{ shortcut }}</kbd></button>
    </div>
    <div class="shell-topbar-actions">
      <BaseButton variant="primary" :disabled="!canManage" @click="$emit('add')"><ShellIcon name="plus" />{{ t('addServer') }}</BaseButton>
      <BaseIconButton :label="t('settings')" :disabled="!settingsEnabled" @click="$emit('settings')"><ShellIcon name="settings" /></BaseIconButton>
      <BaseIconButton :label="t('aboutMauLink')" @click="$emit('about')"><ShellIcon name="info" /></BaseIconButton>
    </div>
  </header>
</template>
