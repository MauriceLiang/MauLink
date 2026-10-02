<script setup lang="ts">
import { CollapsibleRoot, CollapsibleTrigger, CollapsibleContent } from 'reka-ui';
import BaseButton from './BaseButton.vue';
import BaseIcon from './BaseIcon.vue';
defineProps<{ open: boolean; label: string; disabled?: boolean; triggerClass?: string }>();
defineEmits<{ 'update:open': [value: boolean] }>();
</script>
<template><CollapsibleRoot :open="open" :disabled="disabled" @update:open="$emit('update:open',$event)"><CollapsibleTrigger as-child><BaseButton class="base-collapsible-trigger" :class="triggerClass" :disabled="disabled">{{ label }}<BaseIcon name="chevron-down" class="base-collapsible-chevron" /></BaseButton></CollapsibleTrigger><CollapsibleContent class="base-collapsible-content"><slot /></CollapsibleContent></CollapsibleRoot></template>
