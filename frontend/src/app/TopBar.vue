<script setup lang="ts">
import { computed } from "vue";
import { messages } from "../i18n/locale";
import { shellMessages } from "../i18n/shell";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import ShellIcon from "./ShellIcon.vue";
import logoLight from "../assets/maulink-logo-light.png";
import logoDark from "../assets/maulink-logo-dark.png";
const t = messages(shellMessages);
const props = defineProps<{ query: string; shortcut: string; settingsEnabled?: boolean; explorerVisible: boolean }>();
const emit = defineEmits<{ "update:query": [value: string]; about: []; settings: []; palette: []; "toggle-explorer": [] }>();
const explorerLabel = computed(() => t(props.explorerVisible ? "hideServerResources" : "showServerResources"));
</script>

<template>
  <header class="shell-topbar" data-tauri-drag-region>
    <div class="shell-brand-group" data-tauri-drag-region>
      <div class="shell-brand" :aria-label="t('maulinkServerWorkspace')">
        <img class="shell-brand-mark shell-brand-mark-light" :src="logoLight" alt="" aria-hidden="true" width="32" height="32" />
        <img class="shell-brand-mark shell-brand-mark-dark" :src="logoDark" alt="" aria-hidden="true" width="32" height="32" />
        <span>MauLink</span>
      </div>
      <BaseIconButton class="shell-explorer-toggle" :label="explorerLabel" :aria-pressed="explorerVisible" @click="emit('toggle-explorer')"><BaseIcon name="panel-left" /></BaseIconButton>
    </div>
    <div class="shell-search shell-global-search">
      <ShellIcon name="search" />
      <BaseInput id="shell-global-search" :label="t('searchServersOrHosts')" type="search" autocomplete="off"
        :model-value="query" :placeholder="t('searchServersOrHosts2')" @update:model-value="$emit('update:query', $event)" />
      <button type="button" class="palette-trigger" :aria-label="t('commandPalette')" @click="$emit('palette')"><kbd>{{ shortcut }}</kbd></button>
    </div>
    <div class="shell-topbar-actions">
      <BaseIconButton :label="t('settings')" :disabled="!settingsEnabled" @click="$emit('settings')"><ShellIcon name="gear" /></BaseIconButton>
      <BaseIconButton :label="t('aboutMauLink')" @click="$emit('about')"><ShellIcon name="info" /></BaseIconButton>
    </div>
  </header>
</template>
