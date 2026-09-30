<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from "vue";
import BaseIconButton from "./BaseIconButton.vue";

const props = withDefaults(defineProps<{ open: boolean; title: string; busy?: boolean; closeLabel?: string; panelClass?: string }>(), {
  busy: false, closeLabel: "关闭对话框", panelClass: "",
});
const emit = defineEmits<{ close: [] }>();
const titleId = useId();
const dialog = ref<HTMLElement>();
let returnFocus: HTMLElement | null = null;

function focusableElements() {
  // Walk in DOM order so selector-list ordering cannot change the Tab sequence.
  return Array.from(dialog.value?.querySelectorAll<HTMLElement>("*") ?? [])
    .filter(element => element.matches('button, input, select, textarea, a[href], [tabindex]')
      && element.tabIndex >= 0 && !element.matches(':disabled, input[type="hidden"]')
      && !element.closest('[hidden], [inert], [aria-hidden="true"]'))
    .sort((left, right) => (left.tabIndex || Infinity) - (right.tabIndex || Infinity));
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    if (!props.busy) emit("close");
  } else if (event.key === "Tab") {
    const elements = focusableElements();
    const first = elements[0];
    const last = elements.at(-1);
    if (!first) {
      event.preventDefault();
      dialog.value?.focus();
    } else if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog.value)) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
}

watch(() => props.open, async open => {
  if (open) {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    await nextTick();
    if (props.open) (focusableElements()[0] ?? dialog.value)?.focus();
  } else {
    returnFocus?.focus();
    returnFocus = null;
  }
}, { immediate: true });

onBeforeUnmount(() => returnFocus?.focus());
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="base-dialog-overlay">
      <section ref="dialog" class="base-dialog" :class="panelClass" role="dialog" aria-modal="true" :aria-labelledby="titleId"
        :aria-busy="busy" tabindex="-1" @keydown="onKeydown">
        <header class="base-dialog-heading">
          <h2 :id="titleId">{{ title }}</h2>
          <BaseIconButton :label="closeLabel" :disabled="busy" @click="emit('close')">×</BaseIconButton>
        </header>
        <div class="base-dialog-content"><slot /></div>
        <footer v-if="$slots.footer" class="base-dialog-footer"><slot name="footer" /></footer>
      </section>
    </div>
  </Teleport>
</template>
