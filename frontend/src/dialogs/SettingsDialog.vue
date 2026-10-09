<script setup lang="ts">
import BaseSwitch from "../components/base/BaseSwitch.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { ref, watch } from 'vue';
import type { AppIconStyle } from '../../../contracts/v1/AppIconStyle';
import type { AccentColor } from '../../../contracts/v1/AccentColor';
import logoLight from '../assets/app-icon-light.png';
import logoDark from '../assets/app-icon-dark.png';
import type { Theme } from '../../../contracts/v1/Theme';
import type { Language } from '../../../contracts/v1/Language';
import type { UiDensity } from '../../../contracts/v1/UiDensity';
import type { SidebarWidth } from '../../../contracts/v1/SidebarWidth';
import type { ToastPosition } from '../../../contracts/v1/ToastPosition';
import type { TerminalPreferences } from '../terminal/preferences';
import type { BackgroundImagesApi } from '../ipc/background-images';
import { defaultSettings } from '../terminal/preferences';
import BaseDialog from '../components/base/BaseDialog.vue';
import BaseButton from '../components/base/BaseButton.vue';
import BaseIcon from '../components/base/BaseIcon.vue';
import TerminalSettingsDialog from './TerminalSettingsDialog.vue';
import { messages } from '../i18n/locale';
import { settingsMessages } from '../i18n/settings';
import { accentPresetColors } from '../theme/accent';
import GeoIpSettingsSection from '../components/server-overview/GeoIpSettingsSection.vue';
import type { GeoIpApi } from '../ipc/geoip';
import type { CredentialRevealApi } from '../ipc/credential-reveal';
import SecuritySettingsSection from '../components/security/SecuritySettingsSection.vue';
const databaseBusy = ref(false);
const t = messages(settingsMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences; backgroundImages: BackgroundImagesApi; geoip?: GeoIpApi; credentialReveal?: CredentialRevealApi; initialSection?: string }>();
const emit = defineEmits<{ close: []; saved: []; databaseChanged: [] }>();
const iconStyle = ref<AppIconStyle>('light');
const accentColor = ref<AccentColor>('blue');
const customAccentColor = ref('#3B82F6');
const accentOptions: Exclude<AccentColor, 'custom'>[] = ['blue', 'indigo', 'purple', 'green', 'orange', 'red'];
const accentLabelKeys: Record<AccentColor, keyof typeof settingsMessages> = { blue: 'accentBlue', indigo: 'accentIndigo', purple: 'accentPurple', green: 'accentGreen', orange: 'accentOrange', red: 'accentRed', custom: 'accentCustom' };
const toastPositionOptions: ToastPosition[] = ['topLeft', 'topCenter', 'topRight', 'bottomLeft', 'bottomCenter', 'bottomRight'];
const toastPositionLabelKeys: Record<ToastPosition, keyof typeof settingsMessages> = { topLeft: 'toastTopLeft', topCenter: 'toastTopCenter', topRight: 'toastTopRight', bottomLeft: 'toastBottomLeft', bottomCenter: 'toastBottomCenter', bottomRight: 'toastBottomRight' };
const section = ref('general'); const theme = ref<Theme>('system'); const uiDensity = ref<UiDensity>('standard'); const sidebarWidth = ref<SidebarWidth>('standard'); const toastPosition = ref<ToastPosition>('topRight'); const language = ref<Language>('zh-CN'); const confirm = ref(true); const showSizeColumn = ref(true); const showFileSizes = ref(true); const showFolderSizes = ref(false); const terminal = ref(false);
function reset() { const value = props.preferences.record.value?.value ?? defaultSettings; theme.value = value.theme; iconStyle.value = value.appIconStyle; accentColor.value = value.accentColor; customAccentColor.value = value.customAccentColor ?? '#3B82F6'; uiDensity.value = value.uiDensity; sidebarWidth.value = value.sidebarWidth; toastPosition.value = value.toastPosition; language.value = value.language; confirm.value = value.confirmBeforeDisconnect; showSizeColumn.value = value.showSizeColumn; showFileSizes.value = value.showFileSizes; showFolderSizes.value = value.showFolderSizes; }
watch(() => props.open, async open => { if (open) { if (props.initialSection) section.value = props.initialSection; await props.preferences.load(); reset(); } });
async function save() { if (databaseBusy.value || section.value === 'network' || section.value === 'security') return; if (await props.preferences.save({ theme: theme.value, appIconStyle: iconStyle.value, accentColor: accentColor.value, customAccentColor: customAccentColor.value, uiDensity: uiDensity.value, sidebarWidth: sidebarWidth.value, toastPosition: toastPosition.value, language: language.value, confirmBeforeDisconnect: confirm.value, showSizeColumn: showSizeColumn.value, showFileSizes: showFileSizes.value, showFolderSizes: showFolderSizes.value })) { emit('saved'); emit('close'); } }
</script>
<template>
  <BaseDialog :open="open && !terminal" :title="t('settings')" :busy="preferences.busy.value || databaseBusy" panel-class="settings-dialog" @close="emit('close')">
    <nav class="settings-navigation" :aria-label="t('sections')"><BaseButton v-for="item in ['general', 'security', 'appearance', 'files', ...(geoip ? ['network'] as const : []), 'terminal', 'language'] as const" :key="item" :disabled="databaseBusy" :aria-pressed="section === item" @click="section = item">{{ t(item) }}</BaseButton></nav>
    <form v-if="section !== 'security'" id="app-settings-form" class="terminal-settings-form" @submit.prevent="save"><fieldset :disabled="preferences.busy.value || (!preferences.record.value && section !== 'network')">
      <section v-if="section === 'general'"><h3>{{ t('general') }}</h3><BaseSwitch v-model="confirm" :label="t('confirmDisconnect')" :disabled="preferences.busy.value || !preferences.record.value" /><p>{{ t('transferConfirmation') }}</p></section>
      <section v-else-if="section === 'appearance'"><h3>{{ t('appearance') }}</h3><BaseSelect v-model="theme" :label="t('theme')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'system',label:t('system')},{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" /><p>{{ t('themeNote') }}</p><BaseSelect v-model="uiDensity" :label="t('uiDensity')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'standard',label:t('densityStandard')},{value:'compact',label:t('densityCompact')}]" /><BaseSelect v-model="sidebarWidth" :label="t('sidebarWidth')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'narrow',label:t('sidebarNarrow')},{value:'standard',label:t('sidebarStandard')},{value:'wide',label:t('sidebarWide')}]" /><fieldset class="settings-toast-position-fieldset" :disabled="preferences.busy.value || !preferences.record.value"><legend>{{ t('toastPosition') }}</legend><div class="settings-toast-position-options"><label v-for="position in toastPositionOptions" :key="position" class="settings-toast-position-option"><input v-model="toastPosition" type="radio" name="toastPosition" :value="position" /><span>{{ t(toastPositionLabelKeys[position]) }}</span></label></div><div class="settings-toast-preview" :data-position="toastPosition" role="img" :aria-label="t('toastPositionPreview')"><div class="settings-toast-preview-card"><BaseIcon name="success" /><span><strong>{{ t('toastPreviewTitle') }}</strong><small>{{ t('toastPreviewDescription') }}</small></span></div></div><p class="settings-toast-position-note">{{ t('toastPositionNote') }}</p></fieldset><fieldset class="settings-accent-field" :disabled="preferences.busy.value || !preferences.record.value"><legend>{{ t('accentColor') }}</legend><div class="settings-accent-options" role="radiogroup" :aria-label="t('accentColor')"><label v-for="color in accentOptions" :key="color" class="settings-accent-option"><input v-model="accentColor" type="radio" name="accentColor" :value="color" /><span class="settings-accent-swatch" :style="{ '--accent-option-color': accentPresetColors[color] }" aria-hidden="true"></span><span>{{ t(accentLabelKeys[color]) }}</span></label><label class="settings-accent-option"><input v-model="accentColor" type="radio" name="accentColor" value="custom" /><span class="settings-accent-swatch settings-accent-custom-swatch" :style="{ '--accent-option-color': customAccentColor }" aria-hidden="true"></span><span>{{ t('accentCustom') }}</span></label></div><label v-if="accentColor === 'custom'" class="settings-custom-accent"><span>{{ t('customAccentColor') }}</span><input v-model="customAccentColor" type="color" :aria-label="t('customAccentColor')" /><code>{{ customAccentColor.toUpperCase() }}</code></label><p>{{ t('accentColorNote') }}</p></fieldset><BaseSelect v-model="iconStyle" :label="t('appIconStyle')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" /><img class="settings-app-icon-preview" :src="iconStyle === 'dark' ? logoDark : logoLight" :alt="t('appIconPreview')" width="64" height="64" /><p>{{ t('appIconNote') }}</p></section>
      <section v-else-if="section === 'files'"><h3>{{ t('files') }}</h3><div class="settings-file-size-master"><BaseSwitch v-model="showSizeColumn" :label="t('showSizeColumn')" :disabled="preferences.busy.value || !preferences.record.value" /></div><div v-if="showSizeColumn" class="settings-file-size-options"><BaseSwitch v-model="showFileSizes" :label="t('showFileSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><div><BaseSwitch v-model="showFolderSizes" :label="t('showFolderSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><p>{{ t('folderSizeNote') }}</p></div></div></section>
      <GeoIpSettingsSection v-else-if="section === 'network' && geoip" :api="geoip" @changed="emit('databaseChanged')" @busy="databaseBusy = $event" />
      <section v-else-if="section === 'language'"><h3>{{ t('language') }}</h3><BaseSelect v-model="language" :label="t('language')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'zh-CN',label:'中文'},{value:'en',label:'English'}]" /><p>{{ t('languageNote') }}</p></section>
      <section v-else><h3>{{ t('terminal') }}</h3><BaseButton @click="terminal = true">{{ t('terminalSettings') }}</BaseButton><p>{{ t('terminalNote') }}</p></section>
    </fieldset></form>
    <SecuritySettingsSection v-if="open && section === 'security'" :active="open && section === 'security'" :api="credentialReveal" />
    <BaseAlert v-if="preferences.error.value && section !== 'security'">{{ preferences.error.value }}</BaseAlert><BaseButton v-if="preferences.error.value && section !== 'security'" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">{{ t('reload') }}</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value || databaseBusy" @click="emit('close')">{{ t(section === 'network' ? 'done' : 'cancel') }}</BaseButton><BaseButton v-if="section !== 'network' && section !== 'security'" variant="primary" type="submit" form="app-settings-form" :loading="preferences.busy.value" :disabled="!preferences.record.value || databaseBusy">{{ t('save') }}</BaseButton></template>
  </BaseDialog>
  <TerminalSettingsDialog :open="open && terminal" :preferences="preferences" :background-images="backgroundImages" @close="terminal = false" />
</template>
