<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from 'vue';
const props = defineProps<{ label: string; disabled?: boolean; items: { id: string; label: string; disabled?: boolean; title?: string; ariaLabel?: string }[] }>();
const emit = defineEmits<{ action: [id: string] }>();
const open = ref(false); const root = ref<HTMLElement>(); const trigger = ref<HTMLButtonElement>();
function focusable() { return Array.from(root.value?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? []); }
async function show(last = false) { if (props.disabled) return; open.value = true; await nextTick(); (last ? focusable().at(-1) : focusable()[0])?.focus(); }
function close(restore = true) { open.value = false; if (restore) trigger.value?.focus(); }
function keys(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); }
  else if (event.key === 'Tab') close(false);
  else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
    event.preventDefault(); const values = focusable(); const current = values.indexOf(document.activeElement as HTMLButtonElement);
    const index = event.key === 'Home' ? 0 : event.key === 'End' ? values.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + values.length) % values.length; values[index]?.focus();
  }
}
async function action(id: string) { close(); await nextTick(); emit('action', id); }
function outside(event: PointerEvent) { if (event.target instanceof Node && !root.value?.contains(event.target)) close(false); }
document.addEventListener('pointerdown', outside); onBeforeUnmount(() => document.removeEventListener('pointerdown', outside));
</script>
<template><div ref="root" class="file-menu" @keydown="keys"><button ref="trigger" class="file-menu-trigger" :aria-label="label" aria-haspopup="menu" :aria-expanded="open" :disabled="disabled" @click="open ? close() : show()" @keydown.down.stop.prevent="show()" @keydown.up.stop.prevent="show(true)">···</button><div v-if="open" role="menu" :aria-label="label"><button v-for="item in items" :key="item.id" role="menuitem" :data-action="item.id" :disabled="item.disabled" :title="item.title" :aria-label="item.ariaLabel" @click="action(item.id)">{{ item.label }}</button></div></div></template>
