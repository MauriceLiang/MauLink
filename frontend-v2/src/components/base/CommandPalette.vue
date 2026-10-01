<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import BaseDialog from './BaseDialog.vue';
import { messages } from '../../i18n/locale';
import { paletteMessages } from '../../i18n/palette';
import { filterCommands, moveSelection, type PaletteCommand } from '../../app/palette';
const t = messages(paletteMessages);
const props = defineProps<{ open: boolean; commands: PaletteCommand[] }>();
const emit = defineEmits<{ close: []; execute: [id: string] }>();
const query = ref(''); const index = ref(-1); const id = useId();
const filtered = computed(() => filterCommands(props.commands, query.value));
watch(filtered, values => { index.value = moveSelection(values, -1, 1); });
watch(() => props.open, open => { if (open) { query.value = ''; index.value = moveSelection(filtered.value, -1, 1); } }, { immediate: true });
function execute(command?: PaletteCommand) { if (command && !command.disabled) emit('execute', command.id); }
function key(event: KeyboardEvent) {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); index.value = moveSelection(filtered.value, index.value, event.key === 'ArrowDown' ? 1 : -1); document.getElementById(`${id}-${index.value}`)?.scrollIntoView({ block: 'nearest' }); }
  else if (event.key === 'Enter') { event.preventDefault(); execute(filtered.value[index.value]); }
}
</script>
<template><BaseDialog :open="open" :title="t('title')" panel-class="command-palette" initial-focus='[role="combobox"]' @close="emit('close')"><input v-model="query" role="combobox" :aria-label="t('search')" :placeholder="t('search')" aria-autocomplete="list" :aria-expanded="true" :aria-controls="`${id}-results`" :aria-activedescendant="index >= 0 ? `${id}-${index}` : undefined" @keydown="key" /><div :id="`${id}-results`" class="palette-results" role="listbox" :aria-label="t('title')"><button v-for="(command, i) in filtered" :id="`${id}-${i}`" :key="command.id" type="button" role="option" :aria-selected="i === index" :aria-disabled="command.disabled || false" :disabled="command.disabled" tabindex="-1" @mousemove="!command.disabled && (index = i)" @click="execute(command)"><span>{{ command.label }}</span><small>{{ command.meta }}</small></button></div><p v-if="!filtered.length" role="status">{{ t('empty') }}</p><p>{{ t('help') }}</p></BaseDialog></template>
