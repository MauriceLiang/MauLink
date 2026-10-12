<script setup lang="ts">
import { nextTick, useId } from 'vue';
import { RadioGroupItem, RadioGroupRoot } from 'reka-ui';
import type { TerminalThemeMode } from '../../../../contracts/v1/TerminalThemeMode';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';

const props = defineProps<{ modelValue: TerminalThemeMode; label?: string; disabled?: boolean }>();
const emit = defineEmits<{ 'update:modelValue': [value: TerminalThemeMode] }>();
const t = messages(terminalMessages);
const idPrefix = useId();
const modes: { value: TerminalThemeMode; key: keyof typeof terminalMessages }[] = [
  { value: 'followApp', key: 'followApp' },
  { value: 'light', key: 'lightTerminal' },
  { value: 'dark', key: 'darkTerminal' },
  { value: 'customColor', key: 'customColor' },
  { value: 'image', key: 'imageTerminal' },
];

function handleKeydown(event: KeyboardEvent) {
  if (props.disabled) return;
  const currentIndex = modes.findIndex(mode => mode.value === props.modelValue);
  const nextIndex = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? (currentIndex + 1) % modes.length
    : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? (currentIndex - 1 + modes.length) % modes.length
      : event.key === 'Home' ? 0 : event.key === 'End' ? modes.length - 1 : -1;
  if (nextIndex < 0) return;
  event.preventDefault();
  event.stopPropagation();
  const next = modes[nextIndex]!;
  emit('update:modelValue', next.value);
  void nextTick(() => document.getElementById(`${idPrefix}-${next.value}`)?.focus());
}
</script>

<template>
  <fieldset class="terminal-theme-mode-field" :disabled="disabled">
    <legend>{{ label ?? t('themeMode') }}</legend>
    <RadioGroupRoot
      class="terminal-theme-mode-options"
      :model-value="modelValue"
      :aria-label="label ?? t('themeMode')"
      :disabled="disabled"
      @keydown.capture="handleKeydown"
      @update:model-value="emit('update:modelValue', $event as TerminalThemeMode)"
    >
      <label v-for="mode in modes" :key="mode.value" class="terminal-theme-mode-option" :for="`${idPrefix}-${mode.value}`">
        <RadioGroupItem :id="`${idPrefix}-${mode.value}`" class="terminal-theme-mode-radio" :value="mode.value">
          <span class="terminal-theme-mode-indicator" />
        </RadioGroupItem>
        <span>{{ t(mode.key) }}</span>
      </label>
    </RadioGroupRoot>
  </fieldset>
</template>
