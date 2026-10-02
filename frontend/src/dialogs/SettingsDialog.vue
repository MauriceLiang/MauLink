<script setup lang="ts">
import BaseSwitch from "../components/base/BaseSwitch.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { ref, watch } from 'vue';
import type { AppIconStyle } from '../../../contracts/v1/AppIconStyle';
import logoLight from '../assets/app-icon-light.png';
import logoDark from '../assets/app-icon-dark.png';
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
      <section v-if="section === 'general'"><h3>{{ t('general') }}</h3><BaseSwitch v-model="confirm" :label="t('confirmDisconnect')" :disabled="preferences.busy.value || !preferences.record.value" /><p>{{ t('transferConfirmation') }}</p></section>
      <section v-else-if="section === 'appearance'"><h3>{{ t('appearance') }}</h3><BaseSelect v-model="theme" :label="t('theme')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'system',label:t('system')},{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" /><p>{{ t('themeNote') }}</p><BaseSelect v-model="iconStyle" :label="t('appIconStyle')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" /><img class="settings-app-icon-preview" :src="iconStyle === 'dark' ? logoDark : logoLight" :alt="t('appIconPreview')" width="64" height="64" /><p>{{ t('appIconNote') }}</p></section>
      <section v-else-if="section === 'language'"><h3>{{ t('language') }}</h3><BaseSelect v-model="language" :label="t('language')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'zh-CN',label:'中文'},{value:'en',label:'English'}]" /><p>{{ t('languageNote') }}</p></section>
      <section v-else><h3>{{ t('terminal') }}</h3><BaseButton @click="terminal = true">{{ t('terminalSettings') }}</BaseButton><p>{{ t('terminalNote') }}</p></section>
    </fieldset></form>
    <BaseAlert v-if="preferences.error.value">{{ preferences.error.value }}</BaseAlert><BaseButton v-if="preferences.error.value" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">{{ t('reload') }}</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value" @click="emit('close')">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" type="submit" form="app-settings-form" :loading="preferences.busy.value" :disabled="!preferences.record.value">{{ t('save') }}</BaseButton></template>
  </BaseDialog>
  <TerminalSettingsDialog :open="open && terminal" :preferences="preferences" @close="terminal = false" />
</template>
