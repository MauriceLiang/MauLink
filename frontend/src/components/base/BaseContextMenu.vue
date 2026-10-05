<script setup lang="ts">
import { ContextMenuRoot, ContextMenuTrigger, ContextMenuPortal, ContextMenuContent, ContextMenuItem, ContextMenuSeparator } from 'reka-ui';
import type { MenuItem } from './menu';
defineProps<{ label: string; disabled?: boolean; items: MenuItem[] }>();
defineEmits<{ action: [id: string] }>();
</script>
<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child :disabled="disabled"><slot /></ContextMenuTrigger>
    <ContextMenuPortal><ContextMenuContent class="base-menu-content" :aria-label="label" :collision-padding="8" loop>
      <template v-for="item in items" :key="item.id">
        <ContextMenuSeparator v-if="item.separatorBefore" class="base-menu-separator" />
        <ContextMenuItem class="base-menu-item" :class="{'base-menu-item--danger':item.danger}" :data-action="item.id" :disabled="item.disabled" @select="$emit('action',item.id)">{{ item.label }}</ContextMenuItem>
      </template>
    </ContextMenuContent></ContextMenuPortal>
  </ContextMenuRoot>
</template>
