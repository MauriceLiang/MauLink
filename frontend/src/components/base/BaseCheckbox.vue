<script setup lang="ts">
import { useId } from 'vue';
import { CheckboxRoot, CheckboxIndicator } from 'reka-ui';
import BaseIcon from './BaseIcon.vue';
defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>();
defineEmits<{ 'update:modelValue': [value: boolean] }>();
const id=useId();
</script>
<template><div class="base-toggle-field"><CheckboxRoot :id="id" class="base-checkbox" :model-value="modelValue" :disabled="disabled" @update:model-value="$emit('update:modelValue', $event === true)"><CheckboxIndicator><BaseIcon name="check" /></CheckboxIndicator></CheckboxRoot><label :for="id">{{ label }}</label></div></template>
