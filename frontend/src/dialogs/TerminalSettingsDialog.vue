<script setup lang="ts">
import BaseCheckbox from "../components/base/BaseCheckbox.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { messages } from "../i18n/locale";
import { terminalMessages } from "../i18n/terminal";
import { computed, ref, watch } from "vue";
import type { TerminalPreferences } from "../terminal/preferences";
import { defaultSettings } from "../terminal/preferences";
import type { TerminalThemeMode } from "../../../contracts/v1/TerminalThemeMode";
import type { TerminalCustomColors } from "../../../contracts/v1/TerminalCustomColors";
import { resolveTerminalAppearance } from "../terminal/theme";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
const t = messages(terminalMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences }>();
const emit = defineEmits<{ close: [] }>();
const font = ref('monospace'); const size = ref(14); const cursor = ref(defaultSettings.terminalCursorStyle); const scrollback = ref(10000); const copy = ref(false);
const themeMode = ref<TerminalThemeMode>('followApp');
const customColors = ref<TerminalCustomColors>({ ...defaultSettings.terminalCustomColors });
const themeModes: { value: TerminalThemeMode; key: keyof typeof terminalMessages }[] = [
  { value: 'followApp', key: 'followApp' }, { value: 'light', key: 'lightTerminal' },
  { value: 'dark', key: 'darkTerminal' }, { value: 'customColor', key: 'customColor' },
  { value: 'image', key: 'imageTerminal' },
];
const customColorLabels: Record<keyof TerminalCustomColors, keyof typeof terminalMessages> = { background: 'customBackground', foreground: 'customForeground', cursor: 'customCursor', selection: 'customSelection' };
const previewAppearance = computed(() => {
  const current = props.preferences.record.value?.value ?? defaultSettings;
  const systemIsDark = typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches;
  return resolveTerminalAppearance({ ...current, terminalThemeMode: themeMode.value, terminalCustomColors: customColors.value }, current.theme, systemIsDark);
});
function reset() { const value = props.preferences.record.value?.value ?? defaultSettings; font.value = value.terminalFontFamily; size.value = value.terminalFontSize; cursor.value = value.terminalCursorStyle; scrollback.value = value.terminalScrollbackLines; copy.value = props.preferences.copyOnSelect.value; themeMode.value = value.terminalThemeMode; customColors.value = { ...value.terminalCustomColors }; }
watch(() => props.open, async open => { if (open) { await props.preferences.load(); reset(); } });
async function save() { if (await props.preferences.save({ terminalFontFamily: font.value, terminalFontSize: size.value, terminalCursorStyle: cursor.value, terminalScrollbackLines: scrollback.value, terminalThemeMode: themeMode.value, terminalCustomColors: { ...customColors.value } }, copy.value)) emit('close'); }
</script>
<template>
  <BaseDialog :open="open" :title="t('terminalSettings')" :busy="preferences.busy.value" panel-class="terminal-settings-dialog" @close="emit('close')">
    <form id="terminal-settings-form" class="terminal-settings-form" @submit.prevent="save">
      <fieldset :disabled="preferences.busy.value || !preferences.record.value">
        <fieldset class="terminal-theme-mode-field"><legend>{{ t('themeMode') }}</legend><div class="terminal-theme-mode-options" role="radiogroup" :aria-label="t('themeMode')"><label v-for="mode in themeModes" :key="mode.value" class="terminal-theme-mode-option"><input v-model="themeMode" type="radio" name="terminalThemeMode" :value="mode.value" /><span>{{ t(mode.key) }}</span></label></div></fieldset>
        <div v-if="themeMode === 'customColor'" class="terminal-custom-colors"><label v-for="key in ['background', 'foreground', 'cursor', 'selection'] as const" :key="key"><span>{{ t(customColorLabels[key]) }}</span><input v-model="customColors[key]" type="color" :aria-label="t(customColorLabels[key])" /><code>{{ customColors[key].toUpperCase() }}</code></label></div>
        <div class="terminal-preview" role="img" :aria-label="t('terminalPreview')" :style="{ backgroundColor: previewAppearance.theme.background, color: previewAppearance.theme.foreground, fontFamily: font, fontSize: `${size}px`, lineHeight: defaultSettings.terminalLineHeight }"><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> ls -la</div><div>Documents&nbsp; Downloads&nbsp; project</div><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> <span class="terminal-preview-selection" :style="{ backgroundColor: previewAppearance.theme.selectionBackground }">cat README.md</span> <span class="terminal-preview-cursor" :style="{ color: previewAppearance.theme.cursor }">█</span></div></div>
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
