<script setup lang="ts">
defineProps<{ name: 'chevron-down' | 'check' | 'more' | 'x' | 'eye' | 'eye-off' | 'info' | 'warning' | 'error' | 'success' | 'spinner' }>();
</script>
<template>
  <svg class="base-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path v-if="name === 'chevron-down'" d="m6 9 6 6 6-6" />
    <path v-else-if="name === 'check'" d="m5 12 4 4L19 6" />
    <template v-else-if="name === 'more'"><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></template>
    <path v-else-if="name === 'x'" d="m6 6 12 12M6 18 18 6" />
    <template v-else-if="name === 'eye' || name === 'eye-off'"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12" /><circle cx="12" cy="12" r="3" /><path v-if="name === 'eye-off'" d="m3 3 18 18" /></template>
    <path v-else-if="name === 'spinner'" d="M20 12a8 8 0 1 1-8-8" />
    <template v-else><circle cx="12" cy="12" r="9" /><path v-if="name === 'success'" d="m8 12 3 3 5-6" /><path v-else-if="name === 'error'" d="m9 9 6 6m-6 0 6-6" /><path v-else-if="name === 'warning'" d="M12 7v6m0 4h.01" /><path v-else d="M12 11v6m0-10h.01" /></template>
  </svg>
</template>
