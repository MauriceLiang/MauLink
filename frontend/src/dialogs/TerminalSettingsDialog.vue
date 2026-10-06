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
import type { BackgroundImagesApi } from "../ipc/background-images";
import { resolveTerminalTheme } from "../terminal/theme";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { terminalBackgroundImageStyle, terminalBackgroundOverlayStyle } from "../terminal/background";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
const t = messages(terminalMessages);
const props = defineProps<{ open: boolean; preferences: TerminalPreferences; backgroundImages: BackgroundImagesApi }>();
const emit = defineEmits<{ close: [] }>();
const font = ref('monospace'); const size = ref(14); const cursor = ref(defaultSettings.terminalCursorStyle); const scrollback = ref(10000); const copy = ref(false);
const themeMode = ref<TerminalThemeMode>('followApp');
const customColors = ref<TerminalCustomColors>({ ...defaultSettings.terminalCustomColors });
const backgroundSettings = ref<TerminalBackgroundImageSettings>({ ...defaultSettings.terminalBackgroundImage });
const backgroundUrl = ref(''); const backgroundName = ref(''); const imageBusy = ref(false); const imageError = ref('');
const originalImageId = ref<string | null>(null); const importedImageIds = ref<string[]>([]);
const themeModes: { value: TerminalThemeMode; key: keyof typeof terminalMessages }[] = [
  { value: 'followApp', key: 'followApp' }, { value: 'light', key: 'lightTerminal' },
  { value: 'dark', key: 'darkTerminal' }, { value: 'customColor', key: 'customColor' },
  { value: 'image', key: 'imageTerminal' },
];
const customColorLabels: Record<keyof TerminalCustomColors, keyof typeof terminalMessages> = { background: 'customBackground', foreground: 'customForeground', cursor: 'customCursor', selection: 'customSelection' };
const previewAppearance = computed(() => {
  const current = props.preferences.record.value?.value ?? defaultSettings;
  const systemIsDark = typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches;
  return resolveTerminalTheme({ ...current, terminalThemeMode: themeMode.value, terminalCustomColors: customColors.value, terminalBackgroundImage: backgroundSettings.value }, current.theme, systemIsDark);
});
const previewBackgroundStyle = computed(() => terminalBackgroundImageStyle(backgroundUrl.value, backgroundSettings.value));
const previewOverlayStyle = computed(() => terminalBackgroundOverlayStyle(backgroundSettings.value));
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
function opacityLabel(value: number) { return `${value}%`; }
function rangeProgress(value: number, min: number, max: number) {
  const progress = Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100));
  return `${progress}%`;
}
</script>
<template>
  <BaseDialog :open="open" :title="t('terminalSettings')" :busy="preferences.busy.value || imageBusy" panel-class="terminal-settings-dialog" @close="close">
    <form id="terminal-settings-form" class="terminal-settings-form" @submit.prevent="save">
      <fieldset :disabled="preferences.busy.value || imageBusy || !preferences.record.value">
        <fieldset class="terminal-theme-mode-field"><legend>{{ t('themeMode') }}</legend><div class="terminal-theme-mode-options" role="radiogroup" :aria-label="t('themeMode')"><label v-for="mode in themeModes" :key="mode.value" class="terminal-theme-mode-option"><input v-model="themeMode" type="radio" name="terminalThemeMode" :value="mode.value" /><span>{{ t(mode.key) }}</span></label></div></fieldset>
        <div v-if="themeMode === 'customColor'" class="terminal-custom-colors"><label v-for="key in ['background', 'foreground', 'cursor', 'selection'] as const" :key="key"><span>{{ t(customColorLabels[key]) }}</span><input v-model="customColors[key]" type="color" :aria-label="t(customColorLabels[key])" /><code>{{ customColors[key].toUpperCase() }}</code></label></div>
        <section v-if="themeMode === 'image'" class="terminal-background-settings" :aria-label="t('imageTerminal')">
          <div class="terminal-background-preview" role="img" :aria-label="t('terminalPreview')" :style="{ backgroundColor: previewAppearance.theme.background, color: previewAppearance.theme.foreground, fontFamily: font, fontSize: `${size}px`, lineHeight: defaultSettings.terminalLineHeight }"><div v-if="backgroundUrl" class="terminal-background-preview-image" :style="previewBackgroundStyle"></div><div v-if="backgroundUrl" class="terminal-background-preview-overlay" :style="previewOverlayStyle"></div><div class="terminal-background-preview-content"><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> ls -la</div><div>Documents&nbsp; Downloads&nbsp; project</div><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> <span class="terminal-preview-selection" :style="{ backgroundColor: previewAppearance.theme.selectionBackground }">cat README.md</span> <span class="terminal-preview-cursor" :style="{ color: previewAppearance.theme.cursor }">█</span></div></div><span v-if="!backgroundUrl" class="terminal-background-empty">{{ t('imageChoosePrompt') }}</span></div>
          <div class="terminal-custom-colors"><label v-for="key in ['foreground', 'cursor', 'selection'] as const" :key="key"><span>{{ t(customColorLabels[key]) }}</span><input v-model="customColors[key]" type="color" :aria-label="t(customColorLabels[key])" /><code>{{ customColors[key].toUpperCase() }}</code></label></div>
          <div class="terminal-background-actions"><span class="terminal-background-name" :title="backgroundName">{{ backgroundName || t('noImageSelected') }}</span><BaseButton type="button" @click="chooseImage">{{ t('chooseImage') }}</BaseButton><BaseButton type="button" :disabled="!backgroundSettings.imageId" @click="clearImage">{{ t('clearImage') }}</BaseButton></div>
          <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="backgroundSettings.fit" :label="t('imageFit')" :options="[{value:'cover',label:t('fitCover')},{value:'contain',label:t('fitContain')},{value:'stretch',label:t('fitStretch')},{value:'original',label:t('fitOriginal')},{value:'tile',label:t('fitTile')}]" />
          <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="backgroundSettings.position" :label="t('imagePosition')" :options="[{value:'center',label:t('positionCenter')},{value:'top',label:t('positionTop')},{value:'bottom',label:t('positionBottom')},{value:'left',label:t('positionLeft')},{value:'right',label:t('positionRight')}]" />
          <label class="terminal-background-range"><span>{{ t('imageOpacity') }}</span><input v-model.number="backgroundSettings.imageOpacity" type="range" min="10" max="100" step="1" :style="{ '--range-progress': rangeProgress(backgroundSettings.imageOpacity, 10, 100) }" /><output>{{ opacityLabel(backgroundSettings.imageOpacity) }}</output></label>
          <BaseSelect class="terminal-settings-row terminal-settings-select" v-model="backgroundSettings.overlayKind" :label="t('overlayKind')" :options="[{value:'dark',label:t('overlayDark')},{value:'light',label:t('overlayLight')}]" />
          <label class="terminal-background-range"><span>{{ t('overlayOpacity') }}</span><input v-model.number="backgroundSettings.overlayOpacity" type="range" min="0" max="90" step="1" :style="{ '--range-progress': rangeProgress(backgroundSettings.overlayOpacity, 0, 90) }" /><output>{{ opacityLabel(backgroundSettings.overlayOpacity) }}</output></label>
          <label class="terminal-background-range"><span>{{ t('backgroundBlur') }}</span><input v-model.number="backgroundSettings.blurPx" type="range" min="0" max="16" step="1" :style="{ '--range-progress': rangeProgress(backgroundSettings.blurPx, 0, 16) }" /><output>{{ backgroundSettings.blurPx }}px</output></label>
        </section>
        <div v-else class="terminal-preview" role="img" :aria-label="t('terminalPreview')" :style="{ backgroundColor: previewAppearance.theme.background, color: previewAppearance.theme.foreground, fontFamily: font, fontSize: `${size}px`, lineHeight: defaultSettings.terminalLineHeight }"><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> ls -la</div><div>Documents&nbsp; Downloads&nbsp; project</div><div><span class="terminal-preview-prompt" :style="{ color: previewAppearance.theme.cursor }">maurice@server:~$</span> <span class="terminal-preview-selection" :style="{ backgroundColor: previewAppearance.theme.selectionBackground }">cat README.md</span> <span class="terminal-preview-cursor" :style="{ color: previewAppearance.theme.cursor }">█</span></div></div>
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
