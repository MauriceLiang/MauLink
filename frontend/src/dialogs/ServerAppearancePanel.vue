<script setup lang="ts">
import type { AppSettings } from '../../../contracts/v1/AppSettings';
import type { TerminalAppearanceSettings } from '../../../contracts/v1/TerminalAppearanceSettings';
import type { Language } from '../../../contracts/v1/Language';
import { serverText } from '../i18n/servers';
import BaseColorPicker from '../components/base/BaseColorPicker.vue';
import BaseSwitch from '../components/base/BaseSwitch.vue';
import TerminalAppearanceEditor from '../components/terminal/TerminalAppearanceEditor.vue';
import TerminalAppearancePreview from '../components/terminal/TerminalAppearancePreview.vue';

interface ServerAppearanceDraft {
  labelColor: string | null;
  terminalOverrideEnabled: boolean;
  terminalAppearance: TerminalAppearanceSettings;
}

const props = withDefaults(defineProps<{
  modelValue: ServerAppearanceDraft;
  settings: AppSettings;
  language?: Language;
  backgroundUrl?: string;
  backgroundName?: string;
  backgroundBusy?: boolean;
  backgroundError?: string;
  globalBackgroundUrl?: string;
  globalBackgroundBusy?: boolean;
  disabled?: boolean;
}>(), {
  backgroundUrl: '', backgroundName: '', backgroundBusy: false, backgroundError: '',
  globalBackgroundUrl: '', globalBackgroundBusy: false, disabled: false,
});
const emit = defineEmits<{
  'update:modelValue': [value: ServerAppearanceDraft];
  'select-image': [];
  'remove-image': [];
}>();
const t = (key: Parameters<typeof serverText>[0]) => serverText(key, props.language);
const labelPresets = ['#D92D20', '#F79009', '#EAAA08', '#12B76A', '#2E90FA', '#6172F3', '#9E77ED'];

function update(patch: Partial<ServerAppearanceDraft>) {
  if (props.disabled) return;
  const terminalAppearance = patch.terminalAppearance ?? props.modelValue.terminalAppearance;
  emit('update:modelValue', {
    ...props.modelValue,
    ...patch,
    terminalAppearance: {
      themeMode: terminalAppearance.themeMode,
      customColors: { ...terminalAppearance.customColors },
      selectionOpacity: terminalAppearance.selectionOpacity,
      backgroundImage: { ...terminalAppearance.backgroundImage },
    },
  });
}
function updateTerminalAppearance(value: TerminalAppearanceSettings) {
  update({ terminalAppearance: value });
}
</script>

<template>
  <div class="server-appearance-panel">
    <section class="server-appearance-identity">
      <header class="server-appearance-heading">
        <h3>{{ t('serverLabelColor') }}</h3>
        <p>{{ t('serverLabelDescription') }}</p>
      </header>
      <div class="server-appearance-label-colors" role="group" :aria-label="t('serverLabelColor')">
        <button
          type="button"
          class="server-label-color-default"
          :aria-pressed="modelValue.labelColor === null"
          :disabled="disabled"
          @click="update({ labelColor: null })"
        >{{ t('labelColorDefault') }}</button>
        <button
          v-for="color in labelPresets"
          :key="color"
          type="button"
          class="server-label-color-option"
          :style="{ '--label-color': color }"
          :aria-label="`${t('labelColorCustom')} ${color}`"
          :aria-pressed="modelValue.labelColor === color"
          :title="color"
          :disabled="disabled"
          @click="update({ labelColor: color })"
        ><span aria-hidden="true">✓</span></button>
        <BaseColorPicker
          :model-value="modelValue.labelColor ?? labelPresets[0]!"
          :label="t('labelColorCustom')"
          :disabled="disabled"
          :presets="labelPresets"
          @update:model-value="update({ labelColor: $event })"
        />
      </div>
    </section>

    <section class="server-appearance-terminal">
      <div class="server-appearance-override-row">
        <div>
          <h3>{{ t('terminalAppearance') }}</h3>
          <p>{{ t('serverAppearanceDescription') }}</p>
        </div>
        <BaseSwitch
          :model-value="modelValue.terminalOverrideEnabled"
          :label="t('useServerAppearance')"
          :disabled="disabled"
          @update:model-value="update({ terminalOverrideEnabled: $event })"
        />
      </div>
      <template v-if="modelValue.terminalOverrideEnabled">
        <TerminalAppearanceEditor
          :model-value="modelValue.terminalAppearance"
          :settings="settings"
          :background-url="backgroundUrl"
          :background-name="backgroundName"
          :background-busy="backgroundBusy"
          :disabled="disabled"
          @update:model-value="updateTerminalAppearance"
          @select-image="emit('select-image')"
          @remove-image="emit('remove-image')"
        />
        <p v-if="backgroundError" class="server-form-error" role="alert">{{ backgroundError }}</p>
      </template>
      <template v-else>
        <p class="server-appearance-inherited-note">{{ t('serverAppearanceInherited') }}</p>
        <TerminalAppearancePreview
          :settings="settings"
          :background-url="globalBackgroundUrl"
          :loading="globalBackgroundBusy"
          :empty-image="settings.terminalThemeMode === 'image' && !globalBackgroundUrl"
        />
      </template>
    </section>
  </div>
</template>
