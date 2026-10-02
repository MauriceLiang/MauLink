<script setup lang="ts">
import BaseIcon from './BaseIcon.vue';
withDefaults(defineProps<{ kind?: 'info' | 'success' | 'warning' | 'error' }>(), {kind:'error'});
</script>
<template><div class="base-alert" :class="`base-alert--${kind}`" :role="kind === 'error' || kind === 'warning' ? 'alert' : 'status'"><BaseIcon :name="kind" /><div><slot /></div></div></template>
