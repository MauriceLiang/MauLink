<script setup lang="ts">
import { ContextMenuRoot, ContextMenuTrigger, ContextMenuPortal, ContextMenuContent, ContextMenuItem } from 'reka-ui';
import type { MenuItem } from './menu';
defineProps<{ label: string; disabled?: boolean; items: MenuItem[] }>();
defineEmits<{ action: [id: string] }>();
</script>
<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child :disabled="disabled"><slot /></ContextMenuTrigger>
    <ContextMenuPortal><ContextMenuContent class="base-menu-content" :aria-label="label" :collision-padding="8" loop>
      <ContextMenuItem v-for="item in items" :key="item.id" class="base-menu-item" :class="{'base-menu-item--danger':item.danger}" :data-action="item.id" :disabled="item.disabled" @select="$emit('action',item.id)">{{ item.label }}</ContextMenuItem>
    </ContextMenuContent></ContextMenuPortal>
  </ContextMenuRoot>
</template>
