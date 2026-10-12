<script setup lang="ts">
import type { TerminalBackgroundPosition } from '../../../../contracts/v1/TerminalBackgroundPosition';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';

withDefaults(defineProps<{ modelValue: TerminalBackgroundPosition; label?: string; disabled?: boolean }>(), { disabled: false });
const emit = defineEmits<{ 'update:modelValue': [value: TerminalBackgroundPosition] }>();
const t = messages(terminalMessages);
const positions: { value: TerminalBackgroundPosition; key: keyof typeof terminalMessages }[] = [
  { value: 'topLeft', key: 'positionTopLeft' }, { value: 'top', key: 'positionTop' }, { value: 'topRight', key: 'positionTopRight' },
  { value: 'left', key: 'positionLeft' }, { value: 'center', key: 'positionCenter' }, { value: 'right', key: 'positionRight' },
  { value: 'bottomLeft', key: 'positionBottomLeft' }, { value: 'bottom', key: 'positionBottom' }, { value: 'bottomRight', key: 'positionBottomRight' },
];
</script>

<template>
  <fieldset class="terminal-position-field" :disabled="disabled">
    <legend>{{ label ?? t('imagePosition') }}</legend>
    <div class="terminal-position-picker" role="group" :aria-label="label ?? t('imagePosition')">
      <button
        v-for="position in positions"
        :key="position.value"
        type="button"
        class="terminal-position-option"
        :aria-label="t(position.key)"
        :aria-pressed="modelValue === position.value"
        :title="t(position.key)"
        :disabled="disabled"
        @click="emit('update:modelValue', position.value)"
      ><span aria-hidden="true" /></button>
    </div>
  </fieldset>
</template>
