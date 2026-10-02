<script setup lang="ts">
import BaseCheckbox from "../components/base/BaseCheckbox.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { messages } from "../i18n/locale";
import { terminalMessages } from "../i18n/terminal";
import { ref, watch } from "vue";
import type { TerminalPreferences } from "../terminal/preferences";
import { defaultSettings } from "../terminal/preferences";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
const t = messages(terminalMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences }>();
const emit = defineEmits<{ close: [] }>();
const font = ref('monospace'); const size = ref(14); const cursor = ref(defaultSettings.terminalCursorStyle); const scrollback = ref(10000); const copy = ref(false);
function reset() { const value = props.preferences.record.value?.value ?? defaultSettings; font.value = value.terminalFontFamily; size.value = value.terminalFontSize; cursor.value = value.terminalCursorStyle; scrollback.value = value.terminalScrollbackLines; copy.value = props.preferences.copyOnSelect.value; }
watch(() => props.open, async open => { if (open) { await props.preferences.load(); reset(); } });
async function save() { if (await props.preferences.save({ terminalFontFamily: font.value, terminalFontSize: size.value, terminalCursorStyle: cursor.value, terminalScrollbackLines: scrollback.value }, copy.value)) emit('close'); }
</script>
<template>
  <BaseDialog :open="open" :title="t('terminalSettings')" :busy="preferences.busy.value" panel-class="terminal-settings-dialog" @close="emit('close')">
    <form id="terminal-settings-form" class="terminal-settings-form" @submit.prevent="save">
      <fieldset :disabled="preferences.busy.value || !preferences.record.value">
        <label class="terminal-settings-row"><span>{{ t('fontSize') }}</span><input class="base-input" v-model.number="size" type="number" min="8" max="72" step="1" required /></label>
        <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="cursor" :label="t('cursor')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'block',label:t('block')},{value:'bar',label:t('bar')},{value:'underline',label:t('underline')}]" />
        <label class="terminal-settings-row"><span>{{ t('fontFamily') }}</span><input class="base-input" v-model="font" maxlength="128" required /></label>
        <label class="terminal-settings-row"><span>{{ t('scrollbackLines') }}</span><input class="base-input" v-model.number="scrollback" type="number" min="1000" max="100000" step="1" required /></label>
        <div class="terminal-settings-row terminal-settings-copy-row"><BaseCheckbox v-model="copy" :label="t('copyOnSelect')" :disabled="preferences.busy.value || !preferences.record.value" /><p class="connection-muted">{{ t('copyNote') }}</p></div>
      </fieldset>
    </form>
    <BaseAlert v-if="preferences.error.value">{{ preferences.error.value }}</BaseAlert>
    <BaseButton v-if="preferences.error.value" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">{{ t('reloadSettingsDiscardChanges') }}</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value" @click="emit('close')">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" type="submit" form="terminal-settings-form" :disabled="!preferences.record.value" :loading="preferences.busy.value">{{ t('saveSettings') }}</BaseButton></template>
  </BaseDialog>
</template>
