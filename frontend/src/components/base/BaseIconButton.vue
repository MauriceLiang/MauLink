<script setup lang="ts">
import { useForwardExpose } from 'reka-ui';
import BaseButton from './BaseButton.vue';
import type { ButtonSize } from './button.types';
import BaseTooltip from './BaseTooltip.vue';
defineOptions({inheritAttrs:false});
withDefaults(defineProps<{ label: string; disabled?: boolean; size?: ButtonSize }>(), { size: 'md' });
const { forwardRef } = useForwardExpose();
</script>
<template><BaseTooltip :label="label"><BaseButton :ref="forwardRef" v-bind="$attrs" variant="icon" :size="size" class="base-icon-button" :aria-label="label" :disabled="disabled"><span aria-hidden="true"><slot /></span></BaseButton></BaseTooltip></template>
