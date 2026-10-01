<script setup lang="ts">
import { ref, watch } from 'vue';
import type { AppIconStyle } from '../../../contracts/v1/AppIconStyle';
import logoLight from '../assets/maulink-logo-light.png';
import logoDark from '../assets/maulink-logo-dark.png';
import type { Theme } from '../../../contracts/v1/Theme';
import type { Language } from '../../../contracts/v1/Language';
import type { TerminalPreferences } from '../terminal/preferences';
import { defaultSettings } from '../terminal/preferences';
import BaseDialog from '../components/base/BaseDialog.vue';
import BaseButton from '../components/base/BaseButton.vue';
import TerminalSettingsDialog from './TerminalSettingsDialog.vue';
import { messages } from '../i18n/locale';
import { settingsMessages } from '../i18n/settings';
const t = messages(settingsMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences }>();
const emit = defineEmits<{ close: []; saved: [] }>();
const iconStyle = ref<AppIconStyle>('light');
const section = ref('general'); const theme = ref<Theme>('system'); const language = ref<Language>('zh-CN'); const confirm = ref(true); const terminal = ref(false);
function reset() { const value = props.preferences.record.value?.value ?? defaultSettings; theme.value = value.theme; iconStyle.value = value.appIconStyle; language.value = value.language; confirm.value = value.confirmBeforeDisconnect; }
watch(() => props.open, async open => { if (open) { await props.preferences.load(); reset(); } });
async function save() { if (await props.preferences.save({ theme: theme.value, appIconStyle: iconStyle.value, language: language.value, confirmBeforeDisconnect: confirm.value })) { emit('saved'); emit('close'); } }
</script>
<template>
  <BaseDialog :open="open && !terminal" :title="t('settings')" :busy="preferences.busy.value" panel-class="settings-dialog" @close="emit('close')">
    <nav class="settings-navigation" :aria-label="t('sections')"><BaseButton v-for="item in ['general', 'appearance', 'terminal', 'language'] as const" :key="item" :aria-pressed="section === item" @click="section = item">{{ t(item) }}</BaseButton></nav>
    <form id="app-settings-form" class="terminal-settings-form" @submit.prevent="save"><fieldset :disabled="preferences.busy.value || !preferences.record.value">
      <section v-if="section === 'general'"><h3>{{ t('general') }}</h3><label><input v-model="confirm" type="checkbox" />{{ t('confirmDisconnect') }}</label><p>{{ t('transferConfirmation') }}</p></section>
      <section v-else-if="section === 'appearance'"><h3>{{ t('appearance') }}</h3><label>{{ t('theme') }}<select v-model="theme"><option value="system">{{ t('system') }}</option><option value="light">{{ t('light') }}</option><option value="dark">{{ t('dark') }}</option></select></label><p>{{ t('themeNote') }}</p><label>{{ t('appIconStyle') }}<select v-model="iconStyle"><option value="light">{{ t('light') }}</option><option value="dark">{{ t('dark') }}</option></select></label><img class="settings-app-icon-preview" :src="iconStyle === 'dark' ? logoDark : logoLight" :alt="t('appIconPreview')" width="64" height="64" /><p>{{ t('appIconNote') }}</p></section>
      <section v-else-if="section === 'language'"><h3>{{ t('language') }}</h3><label>{{ t('language') }}<select v-model="language"><option value="zh-CN">中文</option><option value="en">English</option></select></label><p>{{ t('languageNote') }}</p></section>
      <section v-else><h3>{{ t('terminal') }}</h3><BaseButton @click="terminal = true">{{ t('terminalSettings') }}</BaseButton><p>{{ t('terminalNote') }}</p></section>
    </fieldset></form>
    <p v-if="preferences.error.value" role="alert">{{ preferences.error.value }}</p><BaseButton v-if="preferences.error.value" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">{{ t('reload') }}</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value" @click="emit('close')">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" type="submit" form="app-settings-form" :loading="preferences.busy.value" :disabled="!preferences.record.value">{{ t('save') }}</BaseButton></template>
  </BaseDialog>
  <TerminalSettingsDialog :open="open && terminal" :preferences="preferences" @close="terminal = false" />
</template>
