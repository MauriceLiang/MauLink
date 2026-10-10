<script setup lang="ts">
// Paths from the existing Lucide icon set; see ../../../LICENSE-lucide.
defineProps<{ name: 'chevron-down' | 'check' | 'more' | 'x' | 'eye' | 'eye-off' | 'info' | 'warning' | 'error' | 'success' | 'spinner' | 'plus' | 'maximize' | 'minimize' | 'settings' | 'eraser' | 'arrow-left' | 'arrow-right' | 'arrow-up' | 'refresh' | 'power' | 'folder' | 'file-text' | 'upload' | 'folder-plus' | 'download' | 'monitor' | 'shield' | 'network' | 'terminal' | 'globe' }>();
</script>
<template>
  <svg class="base-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path v-if="name === 'chevron-down'" d="m6 9 6 6 6-6" />
    <path v-else-if="name === 'check'" d="m5 12 4 4L19 6" />
    <template v-else-if="name === 'more'"><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></template>
    <path v-else-if="name === 'x'" d="m6 6 12 12M6 18 18 6" />
    <path v-else-if="name === 'plus'" d="M5 12h14M12 5v14" />
    <path v-else-if="name === 'arrow-left'" d="M19 12H5m7 7-7-7 7-7" />
    <path v-else-if="name === 'arrow-right'" d="M5 12h14m-7-7 7 7-7 7" />
    <path v-else-if="name === 'arrow-up'" d="M12 19V5m-7 7 7-7 7 7" />
    <template v-else-if="name === 'refresh'"><path d="M20 11a8 8 0 0 0-14.9-4L3 10" /><path d="M3 4v6h6" /><path d="M4 13a8 8 0 0 0 14.9 4L21 14" /><path d="M21 20v-6h-6" /></template>
    <template v-else-if="name === 'power'"><path d="M12 2v10" /><path d="M18.4 6.6a9 9 0 1 1-12.8 0" /></template>
    <path v-else-if="name === 'folder'" d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.7-.9l-.8-1.2A2 2 0 0 0 10 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z" />
    <template v-else-if="name === 'file-text'"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><path d="M14 2v6h6M8 13h8M8 17h8M8 9h2" /></template>
    <template v-else-if="name === 'upload'"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><path d="m17 8-5-5-5 5" /><path d="M12 3v12" /></template>
    <template v-else-if="name === 'folder-plus'"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /><path d="M12 11v6m-3-3h6" /></template>
    <template v-else-if="name === 'download'"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><path d="m7 10 5 5 5-5" /><path d="M12 15V3" /></template>
    <template v-else-if="name === 'monitor'"><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8m-4-4v4" /></template>
    <template v-else-if="name === 'shield'"><path d="M12 22s8-4 8-11V5l-8-3-8 3v6c0 7 8 11 8 11Z" /><path d="m9 12 2 2 4-4" /></template>
    <template v-else-if="name === 'network'"><rect x="9" y="3" width="6" height="5" rx="1" /><rect x="3" y="16" width="6" height="5" rx="1" /><rect x="15" y="16" width="6" height="5" rx="1" /><path d="M12 8v4m-6 4v-2h12v2" /></template>
    <template v-else-if="name === 'terminal'"><path d="m4 17 6-6-6-6" /><path d="M12 19h8" /></template>
    <template v-else-if="name === 'globe'"><circle cx="12" cy="12" r="10" /><path d="M2 12h20M12 2a15 15 0 0 1 0 20M12 2a15 15 0 0 0 0 20" /></template>
    <template v-else-if="name === 'maximize'"><path d="M8 3H5a2 2 0 0 0-2 2v3" /><path d="M21 8V5a2 2 0 0 0-2-2h-3" /><path d="M3 16v3a2 2 0 0 0 2 2h3" /><path d="M16 21h3a2 2 0 0 0 2-2v-3" /></template>
    <template v-else-if="name === 'minimize'"><path d="M8 3v5H3" /><path d="M21 8h-5V3" /><path d="M3 16h5v5" /><path d="M16 21v-5h5" /></template>
    <template v-else-if="name === 'settings'"><path d="M14 17H5M19 7h-9" /><circle cx="17" cy="17" r="3" /><circle cx="7" cy="7" r="3" /></template>
    <template v-else-if="name === 'eraser'"><path d="m7 21 4.545-4.545" /><path d="m4.11 13.33 5.39 5.39a2 2 0 0 0 2.83 0l7.78-7.78a2 2 0 0 0 0-2.83L14.72 2.7a2 2 0 0 0-2.83 0l-7.78 7.78a2 2 0 0 0 0 2.85Z" /><path d="m9 8 6 6M21 21H7" /></template>
    <template v-else-if="name === 'eye' || name === 'eye-off'"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12" /><circle cx="12" cy="12" r="3" /><path v-if="name === 'eye-off'" d="m3 3 18 18" /></template>
    <path v-else-if="name === 'spinner'" d="M20 12a8 8 0 1 1-8-8" />
    <template v-else-if="name === 'info'"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4" /><path d="M12 8h.01" /></template>
    <template v-else><circle cx="12" cy="12" r="9" /><path v-if="name === 'success'" d="m8 12 3 3 5-6" /><path v-else-if="name === 'error'" d="m9 9 6 6m-6 0 6-6" /><path v-else-if="name === 'warning'" d="M12 7v6m0 4h.01" /><path v-else d="M12 11v6m0-10h.01" /></template>
  </svg>
</template>
