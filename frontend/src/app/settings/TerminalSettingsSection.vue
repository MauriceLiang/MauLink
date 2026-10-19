<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import type { TerminalThemeMode } from '../../../../contracts/v1/TerminalThemeMode';
import type { TerminalCustomColors } from '../../../../contracts/v1/TerminalCustomColors';
import type { TerminalBackgroundImageSettings } from '../../../../contracts/v1/TerminalBackgroundImageSettings';
import type { TerminalAppearanceSettings } from '../../../../contracts/v1/TerminalAppearanceSettings';
import type { BackgroundImagesApi } from '../../ipc/background-images';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';
import { defaultSettings } from '../../terminal/preferences';
import { mapError } from '../../errors/mapper';
import { presentError } from '../../errors/presenter';
import BaseAlert from '../../components/base/BaseAlert.vue';
import BaseCheckbox from '../../components/base/BaseCheckbox.vue';
import BaseSelect from '../../components/base/BaseSelect.vue';
import TerminalAppearanceEditor from '../../components/terminal/TerminalAppearanceEditor.vue';

type TerminalSettingsDraft = Pick<AppSettings,
  'terminalFontFamily' | 'terminalFontSize' | 'terminalCursorStyle' | 'terminalScrollbackLines' | 'terminalLineHeight' | 'terminalCursorBlink' |
  'terminalThemeMode' | 'terminalCustomColors' | 'terminalSelectionOpacity' | 'terminalSelectionExtent' | 'terminalBackgroundImage'>;

const props = defineProps<{
  settings: AppSettings;
  savedSettings: AppSettings;
  copyOnSelect: boolean;
  resetKey: number;
  busy: boolean;
  backgroundImages: BackgroundImagesApi;
}>();
const emit = defineEmits<{
  'update:settings': [value: TerminalSettingsDraft];
  'update:copyOnSelect': [value: boolean];
}>();
const t = messages(terminalMessages);
const font = ref('monospace');
const size = ref(14);
const cursor = ref(defaultSettings.terminalCursorStyle);
const scrollback = ref(10000);
const lineHeight = ref(defaultSettings.terminalLineHeight);
const cursorBlink = ref(defaultSettings.terminalCursorBlink);
const copy = ref(false);
const themeMode = ref<TerminalThemeMode>('followApp');
const customColors = ref<TerminalCustomColors>({ ...defaultSettings.terminalCustomColors });
const selectionOpacity = ref(defaultSettings.terminalSelectionOpacity);
const selectionExtent = ref(defaultSettings.terminalSelectionExtent);
const backgroundSettings = ref<TerminalBackgroundImageSettings>({ ...defaultSettings.terminalBackgroundImage });
const backgroundUrl = ref('');
const backgroundName = ref('');
const imageBusy = ref(false);
const imageError = ref('');
const originalImageId = ref<string | null>(null);
const importedImageIds = ref<string[]>([]);
const initialized = ref(false);
const settingsFields = ref<HTMLFieldSetElement | null>(null);
let resetSequence = 0;
let imageRequestSequence = 0;

const appearanceDraft = computed<TerminalAppearanceSettings>({
  get: () => ({ themeMode: themeMode.value, customColors: { ...customColors.value }, selectionOpacity: selectionOpacity.value, selectionExtent: selectionExtent.value, backgroundImage: { ...backgroundSettings.value } }),
  set: value => {
    themeMode.value = value.themeMode;
    customColors.value = { ...value.customColors };
    selectionOpacity.value = value.selectionOpacity;
    selectionExtent.value = value.selectionExtent;
    backgroundSettings.value = { ...value.backgroundImage };
  },
});

function emitDraft() {
  if (!initialized.value) return;
  emit('update:settings', {
    terminalFontFamily: font.value,
    terminalFontSize: size.value,
    terminalCursorStyle: cursor.value,
    terminalScrollbackLines: scrollback.value,
    terminalLineHeight: lineHeight.value,
    terminalCursorBlink: cursorBlink.value,
    terminalThemeMode: themeMode.value,
    terminalCustomColors: { ...customColors.value },
    terminalSelectionOpacity: selectionOpacity.value,
    terminalSelectionExtent: selectionExtent.value,
    terminalBackgroundImage: { ...backgroundSettings.value },
  });
}
watch([font, size, cursor, scrollback, lineHeight, cursorBlink, themeMode, customColors, selectionOpacity, selectionExtent, backgroundSettings], emitDraft, { deep: true });
watch(copy, value => { if (initialized.value) emit('update:copyOnSelect', value); });

async function reset() {
  const sequence = ++resetSequence;
  imageRequestSequence++;
  initialized.value = false;
  const value = props.savedSettings;
  font.value = value.terminalFontFamily;
  size.value = value.terminalFontSize;
  cursor.value = value.terminalCursorStyle;
  scrollback.value = value.terminalScrollbackLines;
  lineHeight.value = value.terminalLineHeight;
  cursorBlink.value = value.terminalCursorBlink;
  copy.value = props.copyOnSelect;
  themeMode.value = value.terminalThemeMode;
  customColors.value = { ...value.terminalCustomColors };
  selectionOpacity.value = value.terminalSelectionOpacity;
  selectionExtent.value = value.terminalSelectionExtent;
  backgroundSettings.value = { ...value.terminalBackgroundImage };
  originalImageId.value = backgroundSettings.value.imageId;
  importedImageIds.value = [];
  backgroundUrl.value = '';
  backgroundName.value = '';
  imageError.value = '';
  if (backgroundSettings.value.imageId) {
    try {
      const result = await props.backgroundImages.resolve(backgroundSettings.value.imageId);
      if (sequence !== resetSequence) return;
      backgroundUrl.value = result.src;
      backgroundName.value = result.asset.fileName;
    } catch (reason) {
      if (sequence === resetSequence) imageError.value = presentError(mapError(reason)).message;
    }
  }
  if (sequence === resetSequence) initialized.value = true;
}
watch(() => props.resetKey, () => { void reset(); }, { immediate: true });

async function cleanup(ids: string[], keep: string | null = null) {
  let failed = false;
  for (const id of new Set(ids)) {
    if (id === keep) continue;
    try { await props.backgroundImages.delete(id); }
    catch (reason) {
      const code = mapError(reason).code;
      if (code !== 'RESOURCE_IN_USE' && code !== 'RESOURCE_NOT_FOUND') { imageError.value = presentError(mapError(reason)).message; failed = true; }
    }
  }
  return !failed;
}
async function chooseImage() {
  if (imageBusy.value) return;
  const sequence = ++imageRequestSequence;
  imageBusy.value = true;
  imageError.value = '';
  try {
    const asset = await props.backgroundImages.select();
    if (!asset) return;
    if (sequence !== imageRequestSequence) { await cleanup([asset.id]); return; }
    if (!importedImageIds.value.includes(asset.id)) importedImageIds.value.push(asset.id);
    backgroundSettings.value.imageId = asset.id;
    backgroundUrl.value = '';
    backgroundName.value = asset.fileName;
    const resolved = await props.backgroundImages.resolve(asset.id);
    if (sequence !== imageRequestSequence) return;
    backgroundSettings.value.imageId = asset.id;
    backgroundUrl.value = resolved.src;
    backgroundName.value = asset.fileName;
  } catch (reason) { if (sequence === imageRequestSequence) imageError.value = presentError(mapError(reason)).message; }
  finally { imageBusy.value = false; }
}
function clearImage() {
  backgroundSettings.value.imageId = null;
  backgroundUrl.value = '';
  backgroundName.value = '';
  if (themeMode.value === 'image') themeMode.value = 'followApp';
}
function validate() {
  imageError.value = '';
  if (settingsFields.value && !settingsFields.value.reportValidity()) return false;
  if (themeMode.value === 'image' && !backgroundSettings.value.imageId) {
    imageError.value = t('imageRequired');
    return false;
  }
  return true;
}
async function discard() {
  if (imageBusy.value || !(await cleanup(importedImageIds.value))) return false;
  await reset();
  return true;
}
async function commit() {
  if (imageBusy.value) return false;
  const staleIds = [...importedImageIds.value, ...(originalImageId.value && originalImageId.value !== backgroundSettings.value.imageId ? [originalImageId.value] : [])];
  if (!(await cleanup(staleIds, backgroundSettings.value.imageId))) return false;
  importedImageIds.value = [];
  originalImageId.value = backgroundSettings.value.imageId;
  return true;
}
defineExpose({ discard, commit, validate, reset, get busy() { return imageBusy.value; } });
</script>

<template>
  <section class="settings-terminal-section" :aria-label="t('terminalSettings')" :aria-busy="busy || imageBusy">
    <fieldset ref="settingsFields" :disabled="busy || imageBusy || !initialized">
      <TerminalAppearanceEditor
        v-model="appearanceDraft"
        :settings="props.settings"
        :background-url="backgroundUrl"
        :background-name="backgroundName"
        :background-busy="imageBusy"
        :disabled="busy || imageBusy || !initialized"
        @select-image="chooseImage"
        @remove-image="clearImage"
      />
      <label class="terminal-settings-row"><span>{{ t('fontSize') }}</span><input class="base-input" v-model.number="size" type="number" min="8" max="72" step="1" required /></label>
      <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="cursor" :label="t('cursor')" :options="[{value:'block',label:t('block')},{value:'bar',label:t('bar')},{value:'underline',label:t('underline')}]" />
      <label class="terminal-settings-row"><span>{{ t('lineHeight') }}</span><input class="base-input" name="terminalLineHeight" v-model.number="lineHeight" type="number" min="1" max="2" step="0.05" required /></label>
      <div class="terminal-settings-row"><BaseCheckbox v-model="cursorBlink" :label="t('cursorBlink')" /></div>
      <label class="terminal-settings-row"><span>{{ t('fontFamily') }}</span><input class="base-input" v-model="font" maxlength="128" required /></label>
      <label class="terminal-settings-row"><span>{{ t('scrollbackLines') }}</span><input class="base-input" v-model.number="scrollback" type="number" min="1000" max="100000" step="1" required /></label>
      <div class="terminal-settings-row terminal-settings-copy-row"><BaseCheckbox v-model="copy" :label="t('copyOnSelect')" /><p class="connection-muted">{{ t('copyNote') }}</p></div>
    </fieldset>
    <BaseAlert v-if="imageError" role="alert">{{ imageError }}</BaseAlert>
  </section>
</template>
