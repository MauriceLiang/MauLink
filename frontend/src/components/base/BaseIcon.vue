<script setup lang="ts">
// Paths from the existing Lucide icon set; see ../../../LICENSE-lucide.
defineProps<{ name: 'chevron-down' | 'check' | 'more' | 'x' | 'eye' | 'eye-off' | 'info' | 'warning' | 'error' | 'success' | 'spinner' | 'plus' | 'maximize' | 'minimize' | 'settings' | 'eraser' | 'arrow-left' | 'power' }>();
</script>
<template>
  <svg class="base-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path v-if="name === 'chevron-down'" d="m6 9 6 6 6-6" />
    <path v-else-if="name === 'check'" d="m5 12 4 4L19 6" />
    <template v-else-if="name === 'more'"><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></template>
    <path v-else-if="name === 'x'" d="m6 6 12 12M6 18 18 6" />
    <path v-else-if="name === 'plus'" d="M5 12h14M12 5v14" />
    <path v-else-if="name === 'arrow-left'" d="M19 12H5m7 7-7-7 7-7" />
    <template v-else-if="name === 'power'"><path d="M12 2v10" /><path d="M18.4 6.6a9 9 0 1 1-12.8 0" /></template>
    <template v-else-if="name === 'maximize'"><path d="M8 3H5a2 2 0 0 0-2 2v3" /><path d="M21 8V5a2 2 0 0 0-2-2h-3" /><path d="M3 16v3a2 2 0 0 0 2 2h3" /><path d="M16 21h3a2 2 0 0 0 2-2v-3" /></template>
    <template v-else-if="name === 'minimize'"><path d="M8 3v5H3" /><path d="M21 8h-5V3" /><path d="M3 16h5v5" /><path d="M16 21v-5h5" /></template>
    <template v-else-if="name === 'settings'"><path d="M14 17H5M19 7h-9" /><circle cx="17" cy="17" r="3" /><circle cx="7" cy="7" r="3" /></template>
    <template v-else-if="name === 'eraser'"><path d="m7 21 4.545-4.545" /><path d="m4.11 13.33 5.39 5.39a2 2 0 0 0 2.83 0l7.78-7.78a2 2 0 0 0 0-2.83L14.72 2.7a2 2 0 0 0-2.83 0l-7.78 7.78a2 2 0 0 0 0 2.85Z" /><path d="m9 8 6 6M21 21H7" /></template>
    <template v-else-if="name === 'eye' || name === 'eye-off'"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12" /><circle cx="12" cy="12" r="3" /><path v-if="name === 'eye-off'" d="m3 3 18 18" /></template>
    <path v-else-if="name === 'spinner'" d="M20 12a8 8 0 1 1-8-8" />
    <template v-else><circle cx="12" cy="12" r="9" /><path v-if="name === 'success'" d="m8 12 3 3 5-6" /><path v-else-if="name === 'error'" d="m9 9 6 6m-6 0 6-6" /><path v-else-if="name === 'warning'" d="M12 7v6m0 4h.01" /><path v-else d="M12 11v6m0-10h.01" /></template>
  </svg>
</template>
