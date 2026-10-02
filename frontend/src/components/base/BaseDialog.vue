<script setup lang="ts">
import { computed, provide, watch } from 'vue';
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, AlertDialogRoot, AlertDialogPortal, AlertDialogOverlay, AlertDialogContent, AlertDialogTitle } from 'reka-ui';
import { locale } from '../../i18n/locale';
import BaseIconButton from './BaseIconButton.vue';
import BaseIcon from './BaseIcon.vue';
import { dialogEscapeKey } from './dialog-context';
const props = withDefaults(defineProps<{ open: boolean; title: string; busy?: boolean; closeLabel?: string; panelClass?: string; initialFocus?: string; alert?: boolean }>(), { busy:false, closeLabel:'', panelClass:'', alert:false });
const emit = defineEmits<{ close: [] }>();
// Both modal types share the same surface and external API; Reka owns their behavior.
const parts = computed(() => props.alert
  ? {Root:AlertDialogRoot,Portal:AlertDialogPortal,Overlay:AlertDialogOverlay,Content:AlertDialogContent,Title:AlertDialogTitle}
  : {Root:DialogRoot,Portal:DialogPortal,Overlay:DialogOverlay,Content:DialogContent,Title:DialogTitle});
let returnFocus: HTMLElement | null = null;
watch(() => props.open, open => { if (open) returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null; }, { immediate:true, flush:'sync' });
provide(dialogEscapeKey, () => update(false));
function update(open: boolean) { if (!open && !props.busy) emit('close'); }
function openAutoFocus(event: Event) {
  const selector = props.initialFocus ?? (props.alert ? '[data-dialog-cancel]' : undefined);
  const target = selector && event.target instanceof HTMLElement ? event.target.querySelector<HTMLElement>(selector) : null;
  if (target && !target.matches(':disabled')) { event.preventDefault(); target.focus({preventScroll:true}); }
}
function closeAutoFocus(event: Event) {
  event.preventDefault();
  if (returnFocus?.isConnected) returnFocus.focus({preventScroll:true});
}
function escape(event: KeyboardEvent) { if (props.busy) event.preventDefault(); }
function outside(event: Event) { if (props.busy) event.preventDefault(); }
</script>
<template>
  <component :is="parts.Root" :open="open" @update:open="update">
    <component :is="parts.Portal">
      <component :is="parts.Overlay" class="base-dialog-overlay" />
      <component :is="parts.Content" class="base-dialog" :class="panelClass" :aria-busy="busy" aria-modal="true" :aria-describedby="undefined" @open-auto-focus="openAutoFocus" @close-auto-focus="closeAutoFocus" @escape-key-down="escape" @interact-outside="outside">
        <header class="base-dialog-heading"><component :is="parts.Title" as="h2">{{ title }}</component>
          <BaseIconButton class="base-dialog-close" :label="closeLabel || (locale === 'en' ? 'Close dialog' : '关闭对话框')" :disabled="busy" @click="update(false)"><BaseIcon name="x" /></BaseIconButton>
        </header>
        <div class="base-dialog-content"><slot /></div>
        <footer v-if="$slots.footer" class="base-dialog-footer"><slot name="footer" /></footer>
      </component>
    </component>
  </component>
</template>
