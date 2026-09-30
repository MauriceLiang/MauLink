<script setup lang="ts">
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import ShellIcon from "./ShellIcon.vue";
defineProps<{ query: string; home: boolean; shortcut: string; canManage: boolean }>();
defineEmits<{ "update:query": [value: string]; home: []; about: []; add: [] }>();
</script>

<template>
  <header class="shell-topbar" data-tauri-drag-region>
    <div class="shell-brand-group" data-tauri-drag-region>
      <button class="shell-brand" type="button" aria-label="MauLink 服务器工作台" @click="$emit('home')">
        <span class="shell-brand-mark" aria-hidden="true">M</span><span>MauLink</span>
      </button>
      <BaseIconButton class="shell-home-button" label="服务器" :aria-current="home ? 'page' : undefined" @click="$emit('home')"><ShellIcon name="house" /></BaseIconButton>
    </div>
    <div class="shell-search shell-global-search">
      <ShellIcon name="search" />
      <BaseInput id="shell-global-search" label="搜索服务器或主机" type="search" autocomplete="off"
        :model-value="query" placeholder="搜索服务器或主机…" @update:model-value="$emit('update:query', $event)" />
      <kbd>{{ shortcut }}</kbd>
    </div>
    <div class="shell-topbar-actions">
      <BaseButton variant="primary" :disabled="!canManage" @click="$emit('add')"><ShellIcon name="plus" />添加服务器</BaseButton>
      <BaseIconButton label="设置" disabled><ShellIcon name="settings" /></BaseIconButton>
      <BaseIconButton label="关于 MauLink" @click="$emit('about')"><ShellIcon name="info" /></BaseIconButton>
    </div>
  </header>
</template>
