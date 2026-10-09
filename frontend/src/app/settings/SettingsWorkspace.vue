<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import type { AccentColor } from '../../../../contracts/v1/AccentColor';
import type { ToastPosition } from '../../../../contracts/v1/ToastPosition';
import { messages } from '../../i18n/locale';
import { settingsMessages } from '../../i18n/settings';
import { defaultSettings, type TerminalPreferences } from '../../terminal/preferences';
import type { BackgroundImagesApi } from '../../ipc/background-images';
import type { GeoIpApi } from '../../ipc/geoip';
import type { CredentialRevealApi } from '../../ipc/credential-reveal';
import { accentPresetColors } from '../../theme/accent';
import BaseAlert from '../../components/base/BaseAlert.vue';
import BaseButton from '../../components/base/BaseButton.vue';
import BaseIcon from '../../components/base/BaseIcon.vue';
import BaseSelect from '../../components/base/BaseSelect.vue';
import BaseSwitch from '../../components/base/BaseSwitch.vue';
import BaseDialog from '../../components/base/BaseDialog.vue';
import GeoIpSettingsSection from '../../components/server-overview/GeoIpSettingsSection.vue';
import SecuritySettingsSection from '../../components/security/SecuritySettingsSection.vue';
import TerminalSettingsSection from './TerminalSettingsSection.vue';
import type { SettingsSection } from './settings-navigation';
import logoLight from '../../assets/app-icon-light.png';
import logoDark from '../../assets/app-icon-dark.png';

const props = defineProps<{
  active: boolean;
  section: SettingsSection;
  preferences: TerminalPreferences;
  backgroundImages: BackgroundImagesApi;
  geoip?: GeoIpApi;
  credentialReveal?: CredentialRevealApi;
}>();
const emit = defineEmits<{ 'update:section': [section: SettingsSection]; leave: []; leaveCancelled: []; saved: []; databaseChanged: []; busyChange: [busy: boolean] }>();
const t = messages(settingsMessages);
const savedSettings = ref<AppSettings>(cloneSettings(defaultSettings));
const draft = reactive<AppSettings>(cloneSettings(defaultSettings));
const savedCopyOnSelect = ref(false);
const copyOnSelect = ref(false);
const resetKey = ref(0);
const databaseBusy = ref(false);
const leaveDialogOpen = ref(false);
const reloadDialogOpen = ref(false);
const terminalSection = ref<InstanceType<typeof TerminalSettingsSection> | null>(null);
const dirty = computed(() => JSON.stringify(draft) !== JSON.stringify(savedSettings.value) || copyOnSelect.value !== savedCopyOnSelect.value);
const regularSection = computed(() => props.section !== 'security' && props.section !== 'network');
const busy = computed(() => props.preferences.busy.value || databaseBusy.value || !!terminalSection.value?.busy);
const accentColorKeys: Record<AccentColor, keyof typeof settingsMessages> = { blue: 'accentBlue', indigo: 'accentIndigo', purple: 'accentPurple', green: 'accentGreen', orange: 'accentOrange', red: 'accentRed', custom: 'accentCustom' };
const toastPositions: ToastPosition[] = ['topLeft', 'topCenter', 'topRight', 'bottomLeft', 'bottomCenter', 'bottomRight'];
const toastPositionKeys: Record<ToastPosition, keyof typeof settingsMessages> = { topLeft: 'toastTopLeft', topCenter: 'toastTopCenter', topRight: 'toastTopRight', bottomLeft: 'toastBottomLeft', bottomCenter: 'toastBottomCenter', bottomRight: 'toastBottomRight' };
const accentOptions: Exclude<AccentColor, 'custom'>[] = ['blue', 'indigo', 'purple', 'green', 'orange', 'red'];
const customAccentColor = computed({ get: () => draft.customAccentColor ?? '#3B82F6', set: (value: string) => { draft.customAccentColor = value; } });
const sectionCopy: Record<SettingsSection, { title: keyof typeof settingsMessages; description: keyof typeof settingsMessages }> = {
  general: { title: 'general', description: 'generalDescription' },
  security: { title: 'securityPrivacy', description: 'securityDescription' },
  appearance: { title: 'appearance', description: 'appearanceDescription' },
  files: { title: 'files', description: 'filesDescription' },
  network: { title: 'network', description: 'networkDescription' },
  terminal: { title: 'terminal', description: 'terminalDescription' },
  language: { title: 'language', description: 'languageDescription' },
};
const title = computed(() => t(sectionCopy[props.section].title));
const description = computed(() => t(sectionCopy[props.section].description));

function cloneSettings(value: AppSettings): AppSettings {
  return { ...value, terminalCustomColors: { ...value.terminalCustomColors }, terminalBackgroundImage: { ...value.terminalBackgroundImage } };
}
function resetDraft() {
  const value = props.preferences.record.value?.value ?? defaultSettings;
  savedSettings.value = cloneSettings(value);
  Object.assign(draft, cloneSettings(value));
  savedCopyOnSelect.value = props.preferences.copyOnSelect.value;
  copyOnSelect.value = savedCopyOnSelect.value;
  resetKey.value++;
}
async function activate() {
  if (!props.preferences.record.value) await props.preferences.load();
  resetDraft();
}
watch(() => props.active, active => { if (active) void activate(); }, { immediate: true });
watch(busy, value => emit('busyChange', value), { immediate: true });

function updateTerminalSettings(value: Pick<AppSettings, 'terminalFontFamily' | 'terminalFontSize' | 'terminalCursorStyle' | 'terminalScrollbackLines' | 'terminalThemeMode' | 'terminalCustomColors' | 'terminalBackgroundImage'>) {
  Object.assign(draft, value);
}
async function discardSettings() {
  if (busy.value) return false;
  if (terminalSection.value && !(await terminalSection.value.discard())) return false;
  Object.assign(draft, cloneSettings(savedSettings.value));
  copyOnSelect.value = savedCopyOnSelect.value;
  resetKey.value++;
  return true;
}
async function saveSettings() {
  if (busy.value || !props.preferences.record.value) return false;
  if (terminalSection.value && !terminalSection.value.validate()) return false;
  const snapshot = cloneSettings(draft);
  if (!(await props.preferences.save(snapshot, copyOnSelect.value))) return false;
  savedSettings.value = cloneSettings(props.preferences.record.value?.value ?? snapshot);
  savedCopyOnSelect.value = props.preferences.copyOnSelect.value;
  emit('saved');
  if (terminalSection.value && !(await terminalSection.value.commit())) return false;
  return true;
}
function requestLeave() {
  if (databaseBusy.value || props.preferences.busy.value || terminalSection.value?.busy) return;
  if (dirty.value) { leaveDialogOpen.value = true; return; }
  void terminalSection.value?.commit().then(ok => { if (ok !== false) emit('leave'); });
}
function cancelLeave() { leaveDialogOpen.value = false; emit('leaveCancelled'); }
async function saveAndLeave() {
  if (await saveSettings()) { leaveDialogOpen.value = false; emit('leave'); }
}
async function discardAndLeave() {
  if (await discardSettings()) { leaveDialogOpen.value = false; emit('leave'); }
}
async function confirmReload() {
  reloadDialogOpen.value = false;
  if (!(await discardSettings())) return;
  await props.preferences.load();
  resetDraft();
}
defineExpose({ requestLeave, get busy() { return busy.value; }, get databaseBusy() { return databaseBusy.value; } });
</script>

<template>
  <section class="settings-workspace" :aria-label="t('settings')" :aria-busy="busy">
    <header class="settings-workspace__header"><div><h1>{{ title }}</h1><p>{{ description }}</p></div></header>
    <div class="settings-workspace__scroll">
      <div class="settings-workspace__body">
        <form id="settings-workspace-form" class="terminal-settings-form" @submit.prevent="saveSettings">
          <fieldset v-if="regularSection && section !== 'terminal'" :disabled="preferences.busy.value || !preferences.record.value">
            <section v-if="section === 'general'" class="settings-workspace-section"><BaseSwitch v-model="draft.confirmBeforeDisconnect" :label="t('confirmDisconnect')" :disabled="preferences.busy.value || !preferences.record.value" /><p>{{ t('transferConfirmation') }}</p></section>
            <section v-else-if="section === 'appearance'" class="settings-workspace-section">
              <BaseSelect v-model="draft.theme" :label="t('theme')" :options="[{value:'system',label:t('system')},{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" /><p>{{ t('themeNote') }}</p>
              <BaseSelect v-model="draft.uiDensity" :label="t('uiDensity')" :options="[{value:'standard',label:t('densityStandard')},{value:'compact',label:t('densityCompact')}]" />
              <BaseSelect v-model="draft.sidebarWidth" :label="t('sidebarWidth')" :options="[{value:'narrow',label:t('sidebarNarrow')},{value:'standard',label:t('sidebarStandard')},{value:'wide',label:t('sidebarWide')}]" />
              <fieldset class="settings-toast-position-fieldset"><legend>{{ t('toastPosition') }}</legend><div class="settings-toast-position-options"><label v-for="position in toastPositions" :key="position" class="settings-toast-position-option"><input v-model="draft.toastPosition" type="radio" name="toastPosition" :value="position" /><span>{{ t(toastPositionKeys[position]) }}</span></label></div><div class="settings-toast-preview" :data-position="draft.toastPosition" role="img" :aria-label="t('toastPositionPreview')"><div class="settings-toast-preview-card"><BaseIcon name="success" /><span><strong>{{ t('toastPreviewTitle') }}</strong><small>{{ t('toastPreviewDescription') }}</small></span></div></div><p class="settings-toast-position-note">{{ t('toastPositionNote') }}</p></fieldset>
              <fieldset class="settings-accent-field"><legend>{{ t('accentColor') }}</legend><div class="settings-accent-options" role="radiogroup" :aria-label="t('accentColor')"><label v-for="color in accentOptions" :key="color" class="settings-accent-option"><input v-model="draft.accentColor" type="radio" name="accentColor" :value="color" /><span class="settings-accent-swatch" :style="{ '--accent-option-color': accentPresetColors[color] }" aria-hidden="true"></span><span>{{ t(accentColorKeys[color]) }}</span></label><label class="settings-accent-option"><input v-model="draft.accentColor" type="radio" name="accentColor" value="custom" /><span class="settings-accent-swatch settings-accent-custom-swatch" :style="{ '--accent-option-color': customAccentColor }" aria-hidden="true"></span><span>{{ t('accentCustom') }}</span></label></div><label v-if="draft.accentColor === 'custom'" class="settings-custom-accent"><span>{{ t('customAccentColor') }}</span><input v-model="customAccentColor" type="color" :aria-label="t('customAccentColor')" /><code>{{ customAccentColor.toUpperCase() }}</code></label><p>{{ t('accentColorNote') }}</p></fieldset>
              <BaseSelect v-model="draft.appIconStyle" :label="t('appIconStyle')" :options="[{value:'light',label:t('light')},{value:'dark',label:t('dark')}]" />
              <img class="settings-app-icon-preview" :src="draft.appIconStyle === 'dark' ? logoDark : logoLight" :alt="t('appIconPreview')" width="64" height="64" /><p>{{ t('appIconNote') }}</p>
            </section>
            <section v-else-if="section === 'files'" class="settings-workspace-section"><div class="settings-file-size-master"><BaseSwitch v-model="draft.showSizeColumn" :label="t('showSizeColumn')" :disabled="preferences.busy.value || !preferences.record.value" /></div><div v-if="draft.showSizeColumn" class="settings-file-size-options"><BaseSwitch v-model="draft.showFileSizes" :label="t('showFileSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><div><BaseSwitch v-model="draft.showFolderSizes" :label="t('showFolderSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><p>{{ t('folderSizeNote') }}</p></div></div></section>
            <section v-else-if="section === 'language'" class="settings-workspace-section"><BaseSelect v-model="draft.language" :label="t('language')" :options="[{value:'zh-CN',label:'中文'},{value:'en',label:'English'}]" /><p>{{ t('languageNote') }}</p></section>
          </fieldset>
          <section v-show="section === 'terminal'" class="settings-workspace-section"><TerminalSettingsSection ref="terminalSection" :settings="draft" :saved-settings="savedSettings" :copy-on-select="copyOnSelect" :reset-key="resetKey" :busy="preferences.busy.value || !preferences.record.value" :background-images="backgroundImages" @update:settings="updateTerminalSettings" @update:copy-on-select="copyOnSelect = $event" /></section>
          <SecuritySettingsSection v-if="section === 'security' && credentialReveal" :active="active" :api="credentialReveal" />
          <GeoIpSettingsSection v-if="section === 'network' && geoip" :api="geoip" @changed="emit('databaseChanged')" @busy="databaseBusy = $event" />
        </form>
        <BaseAlert v-if="preferences.error.value && regularSection" role="alert">{{ preferences.error.value }}</BaseAlert>
        <BaseButton v-if="preferences.error.value && regularSection" variant="secondary" :disabled="preferences.busy.value" @click="reloadDialogOpen = true">{{ t('reload') }}</BaseButton>
      </div>
    </div>
    <footer class="settings-workspace__footer" :data-section="section">
      <template v-if="regularSection">
        <span v-if="dirty" role="status">{{ t('unsavedSettings') }}</span><span v-else aria-live="polite">{{ t('saved') }}</span>
        <div class="settings-workspace__actions"><BaseButton v-if="dirty" variant="secondary" :disabled="busy" @click="discardSettings">{{ t('discardChanges') }}</BaseButton><BaseButton variant="primary" :disabled="!dirty || busy || !preferences.record.value" :loading="preferences.busy.value" @click="saveSettings">{{ t('saveChanges') }}</BaseButton></div>
      </template>
      <span v-else aria-live="polite"></span>
    </footer>
  </section>
  <BaseDialog :open="leaveDialogOpen" :title="t('leaveSettingsTitle')" size="compact" :initial-focus="'[data-dialog-continue]'" :busy="busy" @close="cancelLeave">
    <p>{{ t('leaveSettingsDescription') }}</p>
    <template #footer><BaseButton variant="ghost" data-dialog-continue :disabled="busy" @click="cancelLeave">{{ t('continueEditing') }}</BaseButton><BaseButton variant="secondary" :disabled="busy" @click="discardAndLeave">{{ t('discardAndLeave') }}</BaseButton><BaseButton variant="primary" :disabled="busy" :loading="preferences.busy.value" @click="saveAndLeave">{{ t('saveAndLeave') }}</BaseButton></template>
  </BaseDialog>
  <BaseDialog :open="reloadDialogOpen" :title="t('reloadSettingsTitle')" size="compact" @close="reloadDialogOpen = false">
    <p>{{ t('reloadSettingsDescription') }}</p>
    <template #footer><BaseButton variant="secondary" @click="reloadDialogOpen = false">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" @click="confirmReload">{{ t('confirmReload') }}</BaseButton></template>
  </BaseDialog>
</template>
