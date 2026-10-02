<script setup lang="ts">
import { inject } from "vue";
import { dialogEscapeKey } from "./dialog-context";
import { TooltipProvider, TooltipRoot, TooltipTrigger, TooltipPortal, TooltipContent, injectTooltipProviderContext } from 'reka-ui';
withDefaults(defineProps<{ label: string; side?: 'top' | 'right' | 'bottom' | 'left' }>(), {side:'top'});
const dialogEscape = inject(dialogEscapeKey, undefined);
const provider = injectTooltipProviderContext(null);
</script>
<template>
  <!-- Isolated tests and standalone component previews also retain a provider. -->
  <TooltipProvider v-if="!provider" :delay-duration="600" :skip-delay-duration="300" ignore-non-keyboard-focus><BaseTooltip :label="label" :side="side"><slot /></BaseTooltip></TooltipProvider>
  <TooltipRoot v-else><TooltipTrigger as-child><slot /></TooltipTrigger><TooltipPortal><TooltipContent @escape-key-down="dialogEscape?.()" class="base-tooltip-content" :side="side" :side-offset="6" :collision-padding="8">{{ label }}</TooltipContent></TooltipPortal></TooltipRoot>
</template>
