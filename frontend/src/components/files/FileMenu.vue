<script setup lang="ts">
import { computed } from 'vue';
import BaseDropdownMenu from '../base/BaseDropdownMenu.vue';
import { messages } from '../../i18n/locale';
import { filesMessages } from '../../i18n/files';
const t = messages(filesMessages);
const props = defineProps<{ name: string; disabled: boolean; downloadable: boolean }>();
const emit = defineEmits<{ action: [action: 'download' | 'rename' | 'delete' | 'copy'] }>();
const items = computed(() => [
  {id: 'download', label: t('download'), disabled: !props.downloadable || props.disabled},
  {id: 'copy', label: t('copy'), disabled: props.disabled},
  {id: 'rename', label: t('rename'), disabled: props.disabled},
  {id: 'delete', danger:true, label: t('delete'), disabled: props.disabled},
  {id: 'preview', label: t('preview'), disabled: true, title: t('previewUnavailable')},
]);
function action(id: string) { if (id !== 'preview') emit('action', id as 'download' | 'rename' | 'delete' | 'copy'); }
</script>
<template><BaseDropdownMenu :label="`${name} ${t('actions')}`" :disabled="disabled" :items="items" @action="action" /></template>
