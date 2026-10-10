<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue';
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
const reloadDialogOpen = ref(false);
const terminalSection = ref<InstanceType<typeof TerminalSettingsSection> | null>(null);
const autoSaveReady = ref(false);
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
const terminalSettingKeys = new Set<keyof AppSettings>(['terminalFontFamily', 'terminalFontSize', 'terminalCursorStyle', 'terminalScrollbackLines', 'terminalThemeMode', 'terminalCustomColors', 'terminalBackgroundImage']);
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;
let autoSaveQueued = false;
let activeSave: Promise<boolean> | null = null;

function clearAutoSaveTimer() {
  if (autoSaveTimer) clearTimeout(autoSaveTimer);
  autoSaveTimer = null;
}
function settingsPatch() {
  const patch: Partial<AppSettings> = {};
  const changedKeys = new Set<keyof AppSettings>();
  for (const key of Object.keys(savedSettings.value) as (keyof AppSettings)[]) {
    if (JSON.stringify(draft[key]) !== JSON.stringify(savedSettings.value[key])) {
      Object.assign(patch, { [key]: draft[key] });
      changedKeys.add(key);
    }
  }
  return { patch, changedKeys };
}
function scheduleAutoSave(delay = 350) {
  if (!autoSaveReady.value || !dirty.value) return;
  clearAutoSaveTimer();
  autoSaveTimer = setTimeout(() => {
    autoSaveTimer = null;
    if (props.preferences.busy.value || terminalSection.value?.busy) {
      autoSaveQueued = true;
      return;
    }
    autoSaveQueued = false;
    void saveSettings();
  }, delay);
}

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
  autoSaveReady.value = false;
  clearAutoSaveTimer();
  autoSaveQueued = false;
  if (!props.preferences.record.value) await props.preferences.load();
  resetDraft();
  await nextTick();
  autoSaveReady.value = true;
}
watch(() => props.active, active => { if (active) void activate(); }, { immediate: true });
watch(busy, value => emit('busyChange', value), { immediate: true });
watch(() => [JSON.stringify(draft), copyOnSelect.value], () => {
  if (autoSaveReady.value && dirty.value) scheduleAutoSave();
});
watch(() => props.preferences.busy.value || !!terminalSection.value?.busy, value => {
  if (!value && autoSaveQueued) {
    autoSaveQueued = false;
    scheduleAutoSave(0);
  }
});
onBeforeUnmount(clearAutoSaveTimer);

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
async function persistSettings() {
  if (!autoSaveReady.value || !props.preferences.record.value) return false;
  if (props.preferences.busy.value || terminalSection.value?.busy) {
    autoSaveQueued = true;
    return false;
  }
  const { patch, changedKeys } = settingsPatch();
  const copyChanged = copyOnSelect.value !== savedCopyOnSelect.value;
  if (!changedKeys.size && !copyChanged) return true;
  const terminalChanged = [...changedKeys].some(key => terminalSettingKeys.has(key));
  if (terminalChanged && terminalSection.value && !terminalSection.value.validate()) return false;
  if (!(await props.preferences.save(patch, copyChanged ? copyOnSelect.value : undefined))) return false;
  savedSettings.value = cloneSettings(props.preferences.record.value?.value ?? { ...savedSettings.value, ...patch });
  savedCopyOnSelect.value = props.preferences.copyOnSelect.value;
  if (!props.preferences.error.value) emit('saved');
  if (terminalChanged && terminalSection.value && !(await terminalSection.value.commit())) return false;
  return true;
}
function saveSettings() {
  if (activeSave) return activeSave;
  const task = persistSettings();
  activeSave = task;
  void task.then(
    () => { if (activeSave === task) activeSave = null; },
    () => { if (activeSave === task) activeSave = null; },
  );
  return task;
}
async function requestLeave() {
  if (!autoSaveReady.value || databaseBusy.value || terminalSection.value?.busy || (props.preferences.busy.value && !activeSave)) {
    emit('leaveCancelled');
    return;
  }
  clearAutoSaveTimer();
  if (activeSave && !(await activeSave)) { emit('leaveCancelled'); return; }
  if (dirty.value && !(await saveSettings())) { emit('leaveCancelled'); return; }
  if (dirty.value) { emit('leaveCancelled'); return; }
  if (terminalSection.value && !(await terminalSection.value.commit())) { emit('leaveCancelled'); return; }
  emit('leave');
}
async function confirmReload() {
  reloadDialogOpen.value = false;
  autoSaveReady.value = false;
  clearAutoSaveTimer();
  autoSaveQueued = false;
  if (!(await discardSettings())) {
    autoSaveReady.value = true;
    return;
  }
  await props.preferences.load();
  resetDraft();
  await nextTick();
  autoSaveReady.value = true;
}
defineExpose({ requestLeave, get busy() { return busy.value; }, get databaseBusy() { return databaseBusy.value; } });
</script>

<template>
  <section class="settings-workspace" :aria-label="t('settings')" :aria-busy="busy">
    <header class="settings-workspace__header"><div><h1>{{ title }}</h1><p>{{ description }}</p></div></header>
    <div class="settings-workspace__scroll">
      <div class="settings-workspace__body">
        <form class="terminal-settings-form" @submit.prevent>
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
        <BaseButton v-if="preferences.error.value && regularSection" variant="secondary" :disabled="busy" @click="reloadDialogOpen = true">{{ t('reload') }}</BaseButton>
      </div>
    </div>
  </section>
  <BaseDialog :open="reloadDialogOpen" :title="t('reloadSettingsTitle')" size="compact" @close="reloadDialogOpen = false">
    <p>{{ t('reloadSettingsDescription') }}</p>
    <template #footer><BaseButton variant="secondary" @click="reloadDialogOpen = false">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" @click="confirmReload">{{ t('confirmReload') }}</BaseButton></template>
  </BaseDialog>
</template>
