<script setup lang="ts">
import { computed } from 'vue';
import BaseDropdownMenu from '../base/BaseDropdownMenu.vue';
import BaseContextMenu from '../base/BaseContextMenu.vue';
import { messages } from '../../i18n/locale';
import { filesMessages } from '../../i18n/files';
const t = messages(filesMessages);
const props = withDefaults(defineProps<{ name: string; disabled: boolean; downloadable: boolean; viewable?: boolean; contextMenu?: boolean }>(), { viewable: false, contextMenu: false });
const emit = defineEmits<{ action: [action: 'download' | 'rename' | 'delete' | 'copy' | 'view'] }>();
const items = computed(() => [
  {id: 'download', label: t('download'), disabled: !props.downloadable || props.disabled},
  {id: 'copy', label: t('copy'), disabled: props.disabled},
  {id: 'rename', label: t('rename'), disabled: props.disabled},
  {id: 'delete', danger:true, label: t('delete'), disabled: props.disabled},
  {id: 'view', label: t('viewEdit'), disabled: !props.viewable || props.disabled},
]);
function action(id: string) { emit('action', id as 'download' | 'rename' | 'delete' | 'copy' | 'view'); }
</script>
<template>
  <BaseContextMenu v-if="contextMenu" :label="`${name} ${t('actions')}`" :disabled="disabled" :items="items" @action="action"><slot /></BaseContextMenu>
  <BaseDropdownMenu v-else :label="`${name} ${t('actions')}`" :disabled="disabled" :items="items" @action="action" />
</template>
