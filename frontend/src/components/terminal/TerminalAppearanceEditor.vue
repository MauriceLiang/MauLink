<script setup lang="ts">
import { computed } from 'vue';
import { RadioGroupItem, RadioGroupRoot } from 'reka-ui';
import type { AppSettings } from '../../../../contracts/v1/AppSettings';
import type { TerminalAppearanceSettings } from '../../../../contracts/v1/TerminalAppearanceSettings';
import type { TerminalBackgroundFit } from '../../../../contracts/v1/TerminalBackgroundFit';
import type { TerminalBackgroundOverlayKind } from '../../../../contracts/v1/TerminalBackgroundOverlayKind';
import type { TerminalBackgroundPosition } from '../../../../contracts/v1/TerminalBackgroundPosition';
import type { TerminalCustomColors } from '../../../../contracts/v1/TerminalCustomColors';
import type { TerminalThemeMode } from '../../../../contracts/v1/TerminalThemeMode';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';
import BaseButton from '../base/BaseButton.vue';
import BaseColorPicker from '../base/BaseColorPicker.vue';
import BaseSelect from '../base/BaseSelect.vue';
import BaseSlider from '../base/BaseSlider.vue';
import TerminalBackgroundPositionPicker from './TerminalBackgroundPositionPicker.vue';
import TerminalAppearancePreview from './TerminalAppearancePreview.vue';
import TerminalThemeModeSelector from './TerminalThemeModeSelector.vue';

const props = withDefaults(defineProps<{
  modelValue: TerminalAppearanceSettings;
  settings: AppSettings;
  backgroundUrl?: string;
  backgroundName?: string;
  backgroundBusy?: boolean;
  disabled?: boolean;
}>(), { backgroundUrl: '', backgroundName: '', backgroundBusy: false, disabled: false });
const emit = defineEmits<{
  'update:modelValue': [value: TerminalAppearanceSettings];
  'select-image': [];
  'remove-image': [];
}>();
const t = messages(terminalMessages);
const customColorLabels: Record<keyof TerminalCustomColors, keyof typeof terminalMessages> = {
  background: 'customBackground', foreground: 'customForeground', cursor: 'customCursor', selection: 'customSelection',
};
const previewSettings = computed<AppSettings>(() => ({
  ...props.settings,
  terminalThemeMode: props.modelValue.themeMode,
  terminalCustomColors: { ...props.modelValue.customColors },
  terminalSelectionOpacity: props.modelValue.selectionOpacity,
  terminalSelectionExtent: props.modelValue.selectionExtent,
  terminalBackgroundImage: { ...props.modelValue.backgroundImage },
}));

function change(mutator: (draft: TerminalAppearanceSettings) => void) {
  if (props.disabled) return;
  const next: TerminalAppearanceSettings = {
    themeMode: props.modelValue.themeMode,
    customColors: { ...props.modelValue.customColors },
    selectionOpacity: props.modelValue.selectionOpacity,
    selectionExtent: props.modelValue.selectionExtent,
    backgroundImage: { ...props.modelValue.backgroundImage },
  };
  mutator(next);
  emit('update:modelValue', next);
}
function updateTheme(value: TerminalThemeMode) { change(next => { next.themeMode = value; }); }
function updateColor(key: keyof TerminalCustomColors, value: string) { change(next => { next.customColors[key] = value; }); }
function updateSelectionOpacity(value: number) { change(next => { next.selectionOpacity = value; }); }
function updateSelectionExtent(value: number) { change(next => { next.selectionExtent = value; }); }
function updateImage<K extends keyof TerminalAppearanceSettings['backgroundImage']>(key: K, value: TerminalAppearanceSettings['backgroundImage'][K]) {
  change(next => { next.backgroundImage[key] = value; });
}
function updateFit(value: TerminalBackgroundFit) { updateImage('fit', value); }
function updatePosition(value: TerminalBackgroundPosition) { updateImage('position', value); }
function updateOverlay(value: TerminalBackgroundOverlayKind) { updateImage('overlayKind', value); }
</script>

<template>
  <section class="terminal-appearance-editor" :aria-label="t('appearanceEditor')">
    <TerminalThemeModeSelector :model-value="modelValue.themeMode" :disabled="disabled" @update:model-value="updateTheme" />
    <TerminalAppearancePreview
      :settings="previewSettings"
      :background-url="backgroundUrl"
      :loading="backgroundBusy"
      :empty-image="modelValue.themeMode === 'image' && !backgroundUrl"
    />

    <div class="terminal-appearance-sliders terminal-selection-opacity">
      <BaseSlider :model-value="modelValue.selectionOpacity" :label="t('selectionOpacity')" :min="0" :max="100" unit="%" :disabled="disabled" @update:model-value="updateSelectionOpacity" />
      <p class="connection-muted">{{ t('selectionOpacityNote') }}</p>
    </div>

    <div class="terminal-appearance-sliders terminal-selection-extent">
      <BaseSlider :model-value="modelValue.selectionExtent" :label="t('selectionExtent')" :min="0" :max="100" unit="%" :disabled="disabled" @update:model-value="updateSelectionExtent" />
      <p class="connection-muted">{{ t('selectionExtentNote') }}</p>
    </div>

    <div v-if="modelValue.themeMode === 'customColor'" class="terminal-appearance-colors" :aria-label="t('customColor')">
      <BaseColorPicker
        v-for="key in ['background', 'foreground', 'cursor', 'selection'] as const"
        :key="key"
        :model-value="modelValue.customColors[key]"
        :label="t(customColorLabels[key])"
        :disabled="disabled"
        :presets="['#111318', '#EAECF0', '#3B82F6', '#12B76A', '#F79009', '#D92D20']"
        @update:model-value="updateColor(key, $event)"
      />
    </div>

    <section v-if="modelValue.themeMode === 'image'" class="terminal-appearance-image-settings" :aria-label="t('imageTerminal')">
      <div class="terminal-appearance-image-row">
        <div class="terminal-appearance-image-thumbnail" :style="backgroundUrl ? { backgroundImage: `url(${JSON.stringify(backgroundUrl)})` } : undefined" aria-hidden="true">
          <span v-if="!backgroundUrl">{{ t('noImageSelected') }}</span>
        </div>
        <span class="terminal-appearance-image-name" :title="backgroundName || t('noImageSelected')">{{ backgroundName || t('noImageSelected') }}</span>
        <div class="terminal-appearance-image-actions">
          <BaseButton type="button" :disabled="disabled || backgroundBusy" :loading="backgroundBusy" @click="emit('select-image')">{{ t(modelValue.backgroundImage.imageId ? 'replaceImage' : 'chooseImage') }}</BaseButton>
          <BaseButton v-if="modelValue.backgroundImage.imageId" type="button" :disabled="disabled || backgroundBusy" @click="emit('remove-image')">{{ t('clearImage') }}</BaseButton>
        </div>
      </div>

      <details class="terminal-appearance-color-details" open>
        <summary>{{ t('appearanceTextAndCursorColors') }}</summary>
        <div class="terminal-appearance-colors">
          <BaseColorPicker
            v-for="key in ['foreground', 'cursor', 'selection'] as const"
            :key="key"
            :model-value="modelValue.customColors[key]"
            :label="t(customColorLabels[key])"
            :disabled="disabled"
            :presets="['#EAECF0', '#344054', '#3B82F6', '#12B76A', '#F79009', '#D92D20']"
            @update:model-value="updateColor(key, $event)"
          />
        </div>
      </details>

      <div class="terminal-appearance-image-grid">
        <div class="terminal-appearance-image-controls">
          <BaseSelect
            class="terminal-appearance-fit-field"
            :model-value="modelValue.backgroundImage.fit"
            :label="t('imageFit')"
            :disabled="disabled"
            :options="[
              { value: 'cover', label: t('fitCover') }, { value: 'contain', label: t('fitContain') },
              { value: 'stretch', label: t('fitStretch') }, { value: 'original', label: t('fitOriginal') }, { value: 'tile', label: t('fitTile') },
            ]"
            @update:model-value="updateFit($event as TerminalBackgroundFit)"
          />
          <fieldset class="terminal-appearance-overlay-field" :disabled="disabled">
            <legend>{{ t('overlayKind') }}</legend>
            <RadioGroupRoot
              class="terminal-appearance-segmented"
              :model-value="modelValue.backgroundImage.overlayKind"
              :aria-label="t('overlayKind')"
              :disabled="disabled"
              @update:model-value="updateOverlay($event as TerminalBackgroundOverlayKind)"
            >
              <RadioGroupItem
                v-for="item in [{ value: 'dark', key: 'overlayDark' }, { value: 'light', key: 'overlayLight' }] as const"
                :key="item.value"
                class="terminal-appearance-segment"
                :value="item.value"
                :aria-label="t(item.key)"
              >{{ t(item.key) }}</RadioGroupItem>
            </RadioGroupRoot>
          </fieldset>
        </div>
        <TerminalBackgroundPositionPicker
          :model-value="modelValue.backgroundImage.position"
          :label="t('imagePosition')"
          :disabled="disabled"
          @update:model-value="updatePosition"
        />
      </div>

      <div class="terminal-appearance-sliders">
        <BaseSlider :model-value="modelValue.backgroundImage.imageOpacity" :label="t('imageOpacity')" :min="10" :max="100" unit="%" :disabled="disabled" @update:model-value="updateImage('imageOpacity', $event)" />
        <BaseSlider :model-value="modelValue.backgroundImage.overlayOpacity" :label="t('overlayOpacity')" :min="0" :max="90" unit="%" :disabled="disabled" @update:model-value="updateImage('overlayOpacity', $event)" />
        <BaseSlider :model-value="modelValue.backgroundImage.blurPx" :label="t('backgroundBlur')" :min="0" :max="16" unit="px" :disabled="disabled" @update:model-value="updateImage('blurPx', $event)" />
      </div>
    </section>
  </section>
</template>
