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
import type { TerminalBackgroundImageSettings } from "../../../contracts/v1/TerminalBackgroundImageSettings";
import type { TerminalAppearanceSettings } from "../../../contracts/v1/TerminalAppearanceSettings";
import type { BackgroundImagesApi } from "../ipc/background-images";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
import TerminalAppearanceEditor from "../components/terminal/TerminalAppearanceEditor.vue";
const t = messages(terminalMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences; backgroundImages: BackgroundImagesApi }>();
const emit = defineEmits<{ close: [] }>();
const font = ref('monospace'); const size = ref(14); const cursor = ref(defaultSettings.terminalCursorStyle); const scrollback = ref(10000); const copy = ref(false);
const themeMode = ref<TerminalThemeMode>('followApp');
const customColors = ref<TerminalCustomColors>({ ...defaultSettings.terminalCustomColors });
const backgroundSettings = ref<TerminalBackgroundImageSettings>({ ...defaultSettings.terminalBackgroundImage });
const backgroundUrl = ref(''); const backgroundName = ref(''); const imageBusy = ref(false); const imageError = ref('');
const originalImageId = ref<string | null>(null); const importedImageIds = ref<string[]>([]);
const appearanceDraft = computed<TerminalAppearanceSettings>({
  get: () => ({ themeMode: themeMode.value, customColors: { ...customColors.value }, backgroundImage: { ...backgroundSettings.value } }),
  set: value => {
    themeMode.value = value.themeMode;
    customColors.value = { ...value.customColors };
    backgroundSettings.value = { ...value.backgroundImage };
  },
});
const previewSettings = computed(() => ({
  ...(props.preferences.record.value?.value ?? defaultSettings),
  terminalFontFamily: font.value,
  terminalFontSize: size.value,
}));
async function reset() {
  const value = props.preferences.record.value?.value ?? defaultSettings;
  font.value = value.terminalFontFamily; size.value = value.terminalFontSize; cursor.value = value.terminalCursorStyle;
  scrollback.value = value.terminalScrollbackLines; copy.value = props.preferences.copyOnSelect.value;
  themeMode.value = value.terminalThemeMode; customColors.value = { ...value.terminalCustomColors };
  backgroundSettings.value = { ...value.terminalBackgroundImage }; originalImageId.value = backgroundSettings.value.imageId;
  importedImageIds.value = []; backgroundUrl.value = ''; backgroundName.value = ''; imageError.value = '';
  if (backgroundSettings.value.imageId) {
    try { const result = await props.backgroundImages.resolve(backgroundSettings.value.imageId); backgroundUrl.value = result.src; backgroundName.value = result.asset.fileName; }
    catch (reason) { imageError.value = presentError(mapError(reason)).message; }
  }
}
watch(() => props.open, async open => { if (open) { await props.preferences.load(); await reset(); } });
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
async function close() { if (imageBusy.value) return; if (!(await cleanup(importedImageIds.value))) return; importedImageIds.value = []; emit('close'); }
async function chooseImage() {
  if (imageBusy.value) return;
  imageBusy.value = true; imageError.value = '';
  try {
    const asset = await props.backgroundImages.select();
    if (!asset) return;
    importedImageIds.value.push(asset.id);
    const resolved = await props.backgroundImages.resolve(asset.id);
    backgroundSettings.value.imageId = asset.id; backgroundUrl.value = resolved.src; backgroundName.value = asset.fileName;
  } catch (reason) { imageError.value = presentError(mapError(reason)).message; }
  finally { imageBusy.value = false; }
}
function clearImage() { backgroundSettings.value.imageId = null; backgroundUrl.value = ''; backgroundName.value = ''; if (themeMode.value === 'image') themeMode.value = 'followApp'; }
async function save() {
  if (themeMode.value === 'image' && !backgroundSettings.value.imageId) { imageError.value = t('imageRequired'); return; }
  if (!(await props.preferences.save({ terminalFontFamily: font.value, terminalFontSize: size.value, terminalCursorStyle: cursor.value, terminalScrollbackLines: scrollback.value, terminalThemeMode: themeMode.value, terminalCustomColors: { ...customColors.value }, terminalBackgroundImage: { ...backgroundSettings.value } }, copy.value))) return;
  const staleIds = [...importedImageIds.value, ...(originalImageId.value && originalImageId.value !== backgroundSettings.value.imageId ? [originalImageId.value] : [])];
  if (!(await cleanup(staleIds, backgroundSettings.value.imageId))) return;
  importedImageIds.value = []; originalImageId.value = backgroundSettings.value.imageId;
  emit('close');
}
</script>
<template>
  <BaseDialog :open="open" :title="t('terminalSettings')" :busy="preferences.busy.value || imageBusy" panel-class="terminal-settings-dialog" @close="close">
    <form id="terminal-settings-form" class="terminal-settings-form" @submit.prevent="save">
      <fieldset :disabled="preferences.busy.value || imageBusy || !preferences.record.value">
        <TerminalAppearanceEditor
          v-model="appearanceDraft"
          :settings="previewSettings"
          :background-url="backgroundUrl"
          :background-name="backgroundName"
          :background-busy="imageBusy"
          :disabled="preferences.busy.value || imageBusy || !preferences.record.value"
          @select-image="chooseImage"
          @remove-image="clearImage"
        />
        <label class="terminal-settings-row"><span>{{ t('fontSize') }}</span><input class="base-input" v-model.number="size" type="number" min="8" max="72" step="1" required /></label>
        <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="cursor" :label="t('cursor')" :disabled="preferences.busy.value || !preferences.record.value" :options="[{value:'block',label:t('block')},{value:'bar',label:t('bar')},{value:'underline',label:t('underline')}]" />
        <label class="terminal-settings-row"><span>{{ t('fontFamily') }}</span><input class="base-input" v-model="font" maxlength="128" required /></label>
        <label class="terminal-settings-row"><span>{{ t('scrollbackLines') }}</span><input class="base-input" v-model.number="scrollback" type="number" min="1000" max="100000" step="1" required /></label>
        <div class="terminal-settings-row terminal-settings-copy-row"><BaseCheckbox v-model="copy" :label="t('copyOnSelect')" :disabled="preferences.busy.value || !preferences.record.value" /><p class="connection-muted">{{ t('copyNote') }}</p></div>
      </fieldset>
    </form>
    <BaseAlert v-if="imageError">{{ imageError }}</BaseAlert>
    <BaseAlert v-if="preferences.error.value">{{ preferences.error.value }}</BaseAlert>
    <BaseButton v-if="preferences.error.value" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">{{ t('reloadSettingsDiscardChanges') }}</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value || imageBusy" @click="close">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" type="submit" form="terminal-settings-form" :disabled="!preferences.record.value" :loading="preferences.busy.value">{{ t('saveSettings') }}</BaseButton></template>
  </BaseDialog>
</template>
