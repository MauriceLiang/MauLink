<script setup lang="ts">
import { DropdownMenuRoot, DropdownMenuTrigger, DropdownMenuPortal, DropdownMenuContent, DropdownMenuItem } from 'reka-ui';
import { ref, useId } from 'vue';
import type { MenuItem } from './menu';
import BaseIcon from './BaseIcon.vue';
import BaseIconButton from './BaseIconButton.vue';
defineProps<{ label: string; disabled?: boolean; items: MenuItem[] }>();
const emit = defineEmits<{ action: [id: string] }>();
const triggerId = useId();
const pendingAction = ref<string>();
function select(id: string) { pendingAction.value = id; }
function closeAutoFocus(event: Event) {
  if (pendingAction.value === undefined) return;
  event.preventDefault();
  document.querySelector<HTMLElement>(`[data-menu-trigger="${triggerId}"]`)?.focus();
  const id = pendingAction.value; pendingAction.value = undefined;
  emit('action', id);
}
</script>
<template>
  <DropdownMenuRoot>
    <DropdownMenuTrigger as-child><BaseIconButton :data-menu-trigger="triggerId" class="base-menu-trigger" :label="label" :disabled="disabled"><BaseIcon name="more" /></BaseIconButton></DropdownMenuTrigger>
    <DropdownMenuPortal><DropdownMenuContent class="base-menu-content" align="end" :side-offset="5" :collision-padding="8" loop @close-auto-focus="closeAutoFocus">
      <DropdownMenuItem v-for="item in items" :key="item.id" class="base-menu-item" :class="{'base-menu-item--danger':item.danger}" :data-action="item.id" :disabled="item.disabled" :title="item.title" :aria-label="item.ariaLabel" @select="select(item.id)">{{ item.label }}</DropdownMenuItem>
    </DropdownMenuContent></DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
