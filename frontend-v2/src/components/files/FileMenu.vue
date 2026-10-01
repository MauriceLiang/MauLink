<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from "vue";
const props = defineProps<{ name: string; disabled: boolean; downloadable: boolean }>();
const emit = defineEmits<{ action: [action: 'download' | 'rename' | 'delete' | 'copy'] }>();
const open = ref(false); const root = ref<HTMLElement>(); const trigger = ref<HTMLButtonElement>();
function items() { return Array.from(root.value?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? []); }
async function show(last = false) { if (props.disabled) return; open.value = true; await nextTick(); (last ? items().at(-1) : items()[0])?.focus(); }
function close(restore = true) { open.value = false; if (restore) trigger.value?.focus(); }
function keys(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); }
  else if (event.key === 'Tab') close(false);
  else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
    event.preventDefault(); const values = items(); const current = values.indexOf(document.activeElement as HTMLButtonElement);
    const index = event.key === 'Home' ? 0 : event.key === 'End' ? values.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + values.length) % values.length;
    values[index]?.focus();
  }
}
async function action(value: 'download' | 'rename' | 'delete' | 'copy') { close(); await nextTick(); emit('action', value); }
function outside(event: PointerEvent) { if (event.target instanceof Node && !root.value?.contains(event.target)) close(false); }
document.addEventListener('pointerdown', outside);
onBeforeUnmount(() => document.removeEventListener('pointerdown', outside));
</script>
<template>
  <div ref="root" class="file-menu" @keydown="keys">
    <button ref="trigger" class="file-menu-trigger" :aria-label="`${name} 操作`" aria-haspopup="menu" :aria-expanded="open" :disabled="disabled" @click="open ? close() : show()" @keydown.down.stop.prevent="show()" @keydown.up.stop.prevent="show(true)">···</button>
    <div v-if="open" role="menu" :aria-label="`${name} 操作菜单`">
      <button role="menuitem" :disabled="!downloadable || disabled" @click="action('download')">下载</button>
      <button role="menuitem" :disabled="disabled" @click="action('copy')">复制路径</button>
      <button role="menuitem" :disabled="disabled" @click="action('rename')">重命名</button>
      <button role="menuitem" :disabled="disabled" @click="action('delete')">删除</button>
      <button role="menuitem" disabled title="当前 Core 尚未提供远程文件预览或编辑接口">查看 / 编辑（暂不可用）</button>
    </div>
  </div>
</template>
