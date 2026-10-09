<script setup lang="ts" generic="T extends string | null">
import { computed, useId } from 'vue';
import { SelectRoot, SelectTrigger, SelectValue, SelectIcon, SelectPortal, SelectContent, SelectViewport, SelectItem, SelectItemText, SelectItemIndicator } from 'reka-ui';
import BaseIcon from './BaseIcon.vue';
const props = defineProps<{ modelValue?: T; options: { value: T; label: string; disabled?: boolean }[]; label: string; placeholder?: string; disabled?: boolean; id?: string; error?: string }>();
const emit = defineEmits<{ 'update:modelValue': [value: T] }>();
const fieldId = props.id ?? useId();
// Encode nullable domain values: Reka reserves the empty string for clearing selection.
const encode = (value: T) => value === null ? 'null:' : `value:${value}`;
const value = computed(() => props.modelValue === undefined ? undefined : encode(props.modelValue));
function update(next: unknown) {
  const option = props.options.find(item => encode(item.value) === next);
  if (option && !option.disabled && !props.disabled) emit('update:modelValue', option.value);
}
</script>
<template>
  <div class="base-field">
    <div class="base-field-label">
      <label :for="fieldId">{{ label }}</label>
      <slot name="label-suffix" />
    </div>
    <SelectRoot :model-value="value" :disabled="disabled" @update:model-value="update">
      <SelectTrigger :id="fieldId" class="base-select-trigger" :aria-invalid="!!error" :aria-describedby="error ? `${fieldId}-error` : undefined">
        <SelectValue :placeholder="placeholder" /><SelectIcon><BaseIcon name="chevron-down" /></SelectIcon>
      </SelectTrigger>
      <SelectPortal><SelectContent class="base-select-content" position="popper" :side-offset="5" :collision-padding="8">
        <SelectViewport><SelectItem v-for="option in options" :key="encode(option.value)" class="base-select-item" :value="encode(option.value)" :disabled="option.disabled">
          <SelectItemText>{{ option.label }}</SelectItemText><SelectItemIndicator class="base-select-indicator"><BaseIcon name="check" /></SelectItemIndicator>
        </SelectItem></SelectViewport>
      </SelectContent></SelectPortal>
    </SelectRoot>
    <p v-if="error" :id="`${fieldId}-error`" class="base-field-error">{{ error }}</p>
  </div>
</template>
