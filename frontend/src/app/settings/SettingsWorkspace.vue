<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import type { AccentColor } from '../../../../contracts/v1/AccentColor';
import type { ToastPosition } from '../../../../contracts/v1/ToastPosition';
import type { UpdateChannel } from '../../../../contracts/v1/UpdateChannel';
import { channelForReleaseVersion } from '../../../../tools/release-channel.mjs';
import { messages } from '../../i18n/locale';
import { settingsMessages } from '../../i18n/settings';
import { defaultSettings, type TerminalPreferences } from '../../terminal/preferences';
import type { BackgroundImagesApi } from '../../ipc/background-images';
import type { CredentialRevealApi } from '../../ipc/credential-reveal';
import { accentPresetColors } from '../../theme/accent';
import BaseAlert from '../../components/base/BaseAlert.vue';
import BaseButton from '../../components/base/BaseButton.vue';
import BaseIcon from '../../components/base/BaseIcon.vue';
import BaseSelect from '../../components/base/BaseSelect.vue';
import BaseSwitch from '../../components/base/BaseSwitch.vue';
import BaseDialog from '../../components/base/BaseDialog.vue';
import SecuritySettingsSection from '../../components/security/SecuritySettingsSection.vue';
import UpdateSettingsSection from '../../components/update/UpdateSettingsSection.vue';
import TerminalSettingsSection from './TerminalSettingsSection.vue';
import AppearanceSettingsPreview from './AppearanceSettingsPreview.vue';
import type { SettingsSection } from './settings-navigation';
import type { UpdaterController } from '../../updater/updater-types';
import logoLight from '../../assets/app-icon-light.png';
import logoDark from '../../assets/app-icon-dark.png';

const props = defineProps<{
  active: boolean;
  section: SettingsSection;
  preferences: TerminalPreferences;
  backgroundImages: BackgroundImagesApi;
  updater?: UpdaterController;
  currentVersion?: string;
  updateInstallBusy?: boolean;
  credentialReveal?: CredentialRevealApi;
}>();
const emit = defineEmits<{ 'update:section': [section: SettingsSection]; leave: []; leaveCancelled: []; saved: []; busyChange: [busy: boolean]; updateInstallRequest: []; updateInstallDeferred: [] }>();
const t = messages(settingsMessages);
const savedSettings = ref<AppSettings>(cloneSettings(defaultSettings));
const draft = reactive<AppSettings>(cloneSettings(defaultSettings));
const savedCopyOnSelect = ref(false);
const copyOnSelect = ref(false);
const resetKey = ref(0);
const reloadDialogOpen = ref(false);
const terminalSection = ref<InstanceType<typeof TerminalSettingsSection> | null>(null);
const autoSaveReady = ref(false);
const dirty = computed(() => JSON.stringify(draft) !== JSON.stringify(savedSettings.value) || copyOnSelect.value !== savedCopyOnSelect.value);
const resolvedUpdateChannel = computed(() => resolveUpdateChannel(draft.updateChannel, props.currentVersion ?? '—'));
const updateChannelSaved = computed(() => draft.updateChannel === savedSettings.value.updateChannel);
const regularSection = computed(() => props.section !== 'security');
const busy = computed(() => props.preferences.busy.value || !!terminalSection.value?.busy);
const accentColorKeys: Record<AccentColor, keyof typeof settingsMessages> = { blue: 'accentBlue', indigo: 'accentIndigo', purple: 'accentPurple', green: 'accentGreen', orange: 'accentOrange', red: 'accentRed', custom: 'accentCustom' };
const toastPositions: ToastPosition[] = ['topLeft', 'topCenter', 'topRight', 'bottomLeft', 'bottomCenter', 'bottomRight'];
const toastPositionKeys: Record<ToastPosition, keyof typeof settingsMessages> = { topLeft: 'toastTopLeft', topCenter: 'toastTopCenter', topRight: 'toastTopRight', bottomLeft: 'toastBottomLeft', bottomCenter: 'toastBottomCenter', bottomRight: 'toastBottomRight' };
const accentOptions: Exclude<AccentColor, 'custom'>[] = ['blue', 'indigo', 'purple', 'green', 'orange', 'red'];
const appearanceThemeOptions = [{ value: 'light', label: 'light' }, { value: 'dark', label: 'dark' }, { value: 'system', label: 'system' }] as const;
const appearanceDensityOptions = [{ value: 'compact', label: 'densityCompact' }, { value: 'standard', label: 'densityStandard' }] as const;
const appearanceSidebarOptions = [{ value: 'narrow', label: 'sidebarNarrow' }, { value: 'standard', label: 'sidebarStandard' }, { value: 'wide', label: 'sidebarWide' }] as const;
const appearanceIconOptions = [{ value: 'light', label: 'light' }, { value: 'dark', label: 'dark' }] as const;
const customAccentColor = computed({ get: () => draft.customAccentColor ?? '#3B82F6', set: (value: string) => { draft.customAccentColor = value; } });
const sectionCopy: Record<SettingsSection, { title: keyof typeof settingsMessages; description: keyof typeof settingsMessages }> = {
  general: { title: 'general', description: 'generalDescription' },
  security: { title: 'securityPrivacy', description: 'securityDescription' },
  appearance: { title: 'appearanceDisplay', description: 'appearanceDescription' },
  terminal: { title: 'terminal', description: 'terminalDescription' },
};
const title = computed(() => t(sectionCopy[props.section].title));
const description = computed(() => t(sectionCopy[props.section].description));
const terminalSettingKeys = new Set<keyof AppSettings>(['terminalFontFamily', 'terminalFontSize', 'terminalCursorStyle', 'terminalScrollbackLines', 'terminalLineHeight', 'terminalCursorBlink', 'terminalThemeMode', 'terminalCustomColors', 'terminalSelectionOpacity', 'terminalSelectionExtent', 'terminalBackgroundImage']);
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
      const value = key === 'terminalCustomColors' ? { ...draft.terminalCustomColors }
        : key === 'terminalBackgroundImage' ? { ...draft.terminalBackgroundImage }
          : draft[key];
      Object.assign(patch, { [key]: value });
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
function resolveUpdateChannel(preference: UpdateChannel | null, version: string): UpdateChannel {
  if (preference) return preference;
  try { return channelForReleaseVersion(version); }
  catch { return 'stable'; }
}
function resetDraft() {
  const value = props.preferences.record.value?.value ?? defaultSettings;
  savedSettings.value = cloneSettings(value);
  Object.assign(draft, cloneSettings(value));
  savedCopyOnSelect.value = props.preferences.copyOnSelect.value;
  copyOnSelect.value = savedCopyOnSelect.value;
  resetKey.value++;
}
function restoreAppearanceDefaults() {
  if (!props.preferences.record.value || props.preferences.busy.value) return;
  Object.assign(draft, {
    theme: defaultSettings.theme,
    uiDensity: defaultSettings.uiDensity,
    sidebarWidth: defaultSettings.sidebarWidth,
    appIconStyle: defaultSettings.appIconStyle,
    accentColor: defaultSettings.accentColor,
    customAccentColor: defaultSettings.customAccentColor,
    toastPosition: defaultSettings.toastPosition,
    showSizeColumn: defaultSettings.showSizeColumn,
    showFileSizes: defaultSettings.showFileSizes,
    showFolderSizes: defaultSettings.showFolderSizes,
  });
}
async function activate() {
  autoSaveReady.value = false;
  clearAutoSaveTimer();
  autoSaveQueued = false;
  if (!props.preferences.record.value) await props.preferences.load();
  resetDraft();
  await props.updater?.setChannel(resolvedUpdateChannel.value);
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

function updateTerminalSettings(value: Pick<AppSettings, 'terminalFontFamily' | 'terminalFontSize' | 'terminalCursorStyle' | 'terminalScrollbackLines' | 'terminalLineHeight' | 'terminalCursorBlink' | 'terminalThemeMode' | 'terminalCustomColors' | 'terminalSelectionOpacity' | 'terminalSelectionExtent' | 'terminalBackgroundImage'>) {
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
  const persistedSettings = props.preferences.record.value?.value ?? { ...savedSettings.value, ...patch };
  if (changedKeys.has('updateChannel')) {
    await props.updater?.setChannel(resolveUpdateChannel(persistedSettings.updateChannel, props.currentVersion ?? '—'));
  }
  savedSettings.value = cloneSettings(persistedSettings);
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
async function commitPendingSettings() {
  if (!autoSaveReady.value || terminalSection.value?.busy || (props.preferences.busy.value && !activeSave)) {
    return false;
  }
  clearAutoSaveTimer();
  if (activeSave && !(await activeSave)) return false;
  if (dirty.value && !(await saveSettings())) return false;
  if (dirty.value) return false;
  if (terminalSection.value && !(await terminalSection.value.commit())) return false;
  return true;
}
async function requestLeave() {
  if (!(await commitPendingSettings())) { emit('leaveCancelled'); return; }
  emit('leave');
}
async function prepareForUpdateInstall() { return commitPendingSettings(); }
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
defineExpose({ requestLeave, prepareForUpdateInstall, get busy() { return busy.value; } });
</script>

<template>
  <section class="settings-workspace" :class="{ 'settings-workspace--appearance': section === 'appearance' }" :aria-label="t('settings')" :aria-busy="busy">
    <header v-if="section !== 'appearance'" class="settings-workspace__header"><div><h1>{{ title }}</h1><p>{{ description }}</p></div></header>
    <div class="settings-workspace__scroll">
      <div class="settings-workspace__body" :class="{ 'settings-workspace__body--appearance': section === 'appearance' }">
        <form class="terminal-settings-form" @submit.prevent>
          <fieldset v-if="regularSection && section !== 'terminal'" :disabled="preferences.busy.value || !preferences.record.value">
            <section v-if="section === 'general'" class="settings-workspace-section settings-general-section">
              <div class="settings-preference-group">
                <h2>{{ t('connectionBehavior') }}</h2>
                <BaseSwitch v-model="draft.confirmBeforeDisconnect" :label="t('confirmDisconnect')" :disabled="preferences.busy.value || !preferences.record.value" />
                <p>{{ t('transferConfirmation') }}</p>
              </div>
              <div class="settings-preference-group">
                <h2>{{ t('applicationLanguage') }}</h2>
                <BaseSelect v-model="draft.language" :label="t('language')" :options="[{value:'zh-CN',label:'中文'},{value:'en',label:'English'}]" />
                <p>{{ t('languageNote') }}</p>
              </div>
            </section>
            <section v-else-if="section === 'appearance'" class="settings-appearance-page">
              <header class="settings-appearance-page__header">
                <div>
                  <p class="settings-appearance-page__breadcrumb"><span>{{ t('appearancePreferences') }}</span><span aria-hidden="true">/</span><span>{{ title }}</span></p>
                  <h1>{{ title }}</h1>
                  <p class="settings-appearance-page__description">{{ t('appearancePageSubtitle') }}</p>
                </div>
                <div class="settings-appearance-page__actions">
                  <span class="settings-appearance-page__live"><i aria-hidden="true"></i>{{ t('changesApplyImmediately') }}</span>
                  <BaseButton variant="secondary" :disabled="preferences.busy.value || !preferences.record.value" @click="restoreAppearanceDefaults"><BaseIcon name="refresh" />{{ t('restoreDefaults') }}</BaseButton>
                </div>
              </header>

              <div class="settings-appearance-page__grid">
                <div class="settings-appearance-page__controls">
                  <section class="settings-appearance-group">
                    <header class="settings-appearance-group__heading"><h2>{{ t('appearanceStyle') }}</h2><span>{{ t('appearanceStyleHint') }}</span></header>
                    <div class="settings-appearance-card">
                      <div class="settings-appearance-row">
                        <div class="settings-appearance-copy"><strong>{{ t('theme') }}</strong><small>{{ t('themeNote') }}</small></div>
                        <div class="settings-appearance-segmented" role="radiogroup" :aria-label="t('theme')">
                          <label v-for="option in appearanceThemeOptions" :key="option.value"><input v-model="draft.theme" type="radio" name="appearanceTheme" :value="option.value" /><span>{{ t(option.label) }}</span></label>
                        </div>
                      </div>
                      <div class="settings-appearance-row settings-appearance-row--accent">
                        <div class="settings-appearance-copy"><strong>{{ t('accentColor') }}</strong><small>{{ t('accentColorNote') }}</small></div>
                        <div class="settings-appearance-accent-options" role="radiogroup" :aria-label="t('accentColor')">
                          <label v-for="color in accentOptions" :key="color" class="settings-appearance-accent-option" :title="t(accentColorKeys[color])"><input v-model="draft.accentColor" type="radio" name="appearanceAccent" :value="color" /><span class="settings-appearance-accent-swatch" :style="{ '--accent-option-color': accentPresetColors[color] }"></span><span class="settings-sr-only">{{ t(accentColorKeys[color]) }}</span></label>
                          <label class="settings-appearance-accent-custom"><input v-model="draft.accentColor" type="radio" name="appearanceAccent" value="custom" /><BaseIcon name="plus" /><span>{{ t('accentCustom') }}</span></label>
                          <label v-if="draft.accentColor === 'custom'" class="settings-appearance-custom-color"><span>{{ t('customAccentColor') }}</span><input v-model="customAccentColor" type="color" :aria-label="t('customAccentColor')" /><code>{{ customAccentColor.toUpperCase() }}</code></label>
                        </div>
                      </div>
                      <div class="settings-appearance-row">
                        <div class="settings-appearance-copy"><strong>{{ t('appIconStyle') }}</strong><small>{{ t('appIconNote') }}</small></div>
                        <div class="settings-appearance-icon-control">
                          <div class="settings-appearance-segmented" role="radiogroup" :aria-label="t('appIconStyle')">
                            <label v-for="option in appearanceIconOptions" :key="option.value"><input v-model="draft.appIconStyle" type="radio" name="appearanceIcon" :value="option.value" /><span>{{ t(option.label) }}</span></label>
                          </div>
                          <img :src="draft.appIconStyle === 'dark' ? logoDark : logoLight" :alt="t('appIconPreview')" width="36" height="36" />
                        </div>
                      </div>
                    </div>
                  </section>

                  <section class="settings-appearance-group">
                    <header class="settings-appearance-group__heading"><h2>{{ t('appearanceLayout') }}</h2><span>{{ t('appearanceLayoutHint') }}</span></header>
                    <div class="settings-appearance-card">
                      <div class="settings-appearance-row">
                        <div class="settings-appearance-copy"><strong>{{ t('uiDensity') }}</strong></div>
                        <div class="settings-appearance-segmented" role="radiogroup" :aria-label="t('uiDensity')">
                          <label v-for="option in appearanceDensityOptions" :key="option.value"><input v-model="draft.uiDensity" type="radio" name="appearanceDensity" :value="option.value" /><span>{{ t(option.label) }}</span></label>
                        </div>
                      </div>
                      <div class="settings-appearance-row">
                        <div class="settings-appearance-copy"><strong>{{ t('sidebarWidth') }}</strong></div>
                        <div class="settings-appearance-segmented" role="radiogroup" :aria-label="t('sidebarWidth')">
                          <label v-for="option in appearanceSidebarOptions" :key="option.value"><input v-model="draft.sidebarWidth" type="radio" name="appearanceSidebar" :value="option.value" /><span>{{ t(option.label) }}</span></label>
                        </div>
                      </div>
                    </div>
                  </section>

                  <section class="settings-appearance-group">
                    <header class="settings-appearance-group__heading"><h2>{{ t('fileListDisplay') }}</h2><span>{{ t('appearanceFilesHint') }}</span></header>
                    <div class="settings-appearance-card settings-appearance-file-card">
                      <div class="settings-appearance-file-row"><div class="settings-appearance-file-copy"><BaseSwitch v-model="draft.showSizeColumn" :label="t('showSizeColumn')" :disabled="preferences.busy.value || !preferences.record.value" /><small>{{ t('appearanceSizeColumnHint') }}</small></div></div>
                      <div v-if="draft.showSizeColumn" class="settings-appearance-file-row"><div class="settings-appearance-file-copy"><BaseSwitch v-model="draft.showFileSizes" :label="t('showFileSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><small>{{ t('appearanceFileSizeHint') }}</small></div></div>
                      <div v-if="draft.showSizeColumn" class="settings-appearance-file-row"><div class="settings-appearance-file-copy"><BaseSwitch v-model="draft.showFolderSizes" :label="t('showFolderSizes')" :disabled="preferences.busy.value || !preferences.record.value" /><small>{{ t('folderSizeNote') }}</small></div></div>
                    </div>
                  </section>

                  <section class="settings-appearance-group">
                    <header class="settings-appearance-group__heading"><h2>{{ t('appearanceNotifications') }}</h2><span>{{ t('toastPositionNote') }}</span></header>
                    <fieldset class="settings-appearance-card settings-appearance-toast-card" :aria-label="t('toastPosition')">
                      <legend class="settings-sr-only">{{ t('toastPosition') }}</legend>
                      <div class="settings-toast-position-options"><label v-for="position in toastPositions" :key="position" class="settings-toast-position-option"><input v-model="draft.toastPosition" type="radio" name="toastPosition" :value="position" /><span>{{ t(toastPositionKeys[position]) }}</span></label></div>
                    </fieldset>
                  </section>
                </div>
                <AppearanceSettingsPreview :settings="draft" />
              </div>
            </section>
          </fieldset>
          <UpdateSettingsSection v-if="section === 'general' && updater" :controller="updater" :current-version="currentVersion ?? '—'" :channel="resolvedUpdateChannel" :channel-saved="updateChannelSaved" :install-busy="updateInstallBusy || preferences.busy.value || !preferences.record.value" @update:channel="draft.updateChannel = $event" @install-request="emit('updateInstallRequest')" @install-deferred="emit('updateInstallDeferred')" />
          <section v-show="section === 'terminal'" class="settings-workspace-section"><TerminalSettingsSection ref="terminalSection" :settings="draft" :saved-settings="savedSettings" :copy-on-select="copyOnSelect" :reset-key="resetKey" :busy="preferences.busy.value || !preferences.record.value" :background-images="backgroundImages" @update:settings="updateTerminalSettings" @update:copy-on-select="copyOnSelect = $event" /></section>
          <SecuritySettingsSection v-if="section === 'security' && credentialReveal" :active="active" :api="credentialReveal" />
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
