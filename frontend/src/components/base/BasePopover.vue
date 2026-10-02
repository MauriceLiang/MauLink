<script setup lang="ts">
import { useId } from "vue";
import { PopoverRoot, PopoverTrigger, PopoverPortal, PopoverContent } from 'reka-ui';
const titleId=useId();
defineProps<{ title: string; side?: 'top' | 'right' | 'bottom' | 'left' }>();
</script>
<template><PopoverRoot><PopoverTrigger as-child><slot name="trigger" /></PopoverTrigger><PopoverPortal><PopoverContent :aria-labelledby="titleId" class="base-popover-content" :side="side ?? 'bottom'" :side-offset="6" :collision-padding="8"><h3 :id="titleId">{{ title }}</h3><slot /></PopoverContent></PopoverPortal></PopoverRoot></template>
