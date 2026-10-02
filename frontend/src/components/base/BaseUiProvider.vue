<script setup lang="ts">
import { TooltipProvider } from 'reka-ui';
</script>
<template><TooltipProvider :delay-duration="600" :skip-delay-duration="300" ignore-non-keyboard-focus><slot /></TooltipProvider></template>
