<script setup lang="ts">
import { useId } from 'vue';
import { SwitchRoot, SwitchThumb } from 'reka-ui';
defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>();
defineEmits<{ 'update:modelValue': [value: boolean] }>();
const id=useId();
</script>
<template><div class="base-toggle-field"><SwitchRoot :id="id" class="base-switch" :model-value="modelValue" :disabled="disabled" @update:model-value="$emit('update:modelValue', $event)"><SwitchThumb class="base-switch-thumb" /></SwitchRoot><label :for="id">{{ label }}</label></div></template>
