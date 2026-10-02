<script setup lang="ts">
import { useForwardExpose } from 'reka-ui';
import BaseButton from './BaseButton.vue';
import BaseTooltip from './BaseTooltip.vue';
defineOptions({inheritAttrs:false});
defineProps<{ label: string; disabled?: boolean }>();
const { forwardRef } = useForwardExpose();
</script>
<template><BaseTooltip :label="label"><BaseButton :ref="forwardRef" v-bind="$attrs" class="base-icon-button" :aria-label="label" :disabled="disabled"><span aria-hidden="true"><slot /></span></BaseButton></BaseTooltip></template>
