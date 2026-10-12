<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui';
import { messages } from '../../i18n/locale';
import { terminalMessages } from '../../i18n/terminal';

const props = withDefaults(defineProps<{
  modelValue: string;
  label: string;
  disabled?: boolean;
  presets?: string[];
}>(), { disabled: false, presets: () => [] });
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();
const t = messages(terminalMessages);
const open = ref(false);
const candidate = ref(props.modelValue.toUpperCase());
const valid = computed(() => /^#[\da-f]{6}$/i.test(candidate.value));
const safeValue = computed(() => /^#[\da-f]{6}$/i.test(props.modelValue) ? props.modelValue : '#000000');

watch(() => props.modelValue, value => { candidate.value = value.toUpperCase(); });

function normalize(value: string) {
  return /^#[\da-f]{6}$/i.test(value) ? value.toUpperCase() : null;
}
function updateCandidate(value: string) {
  candidate.value = value;
  const color = normalize(value);
  if (color && !props.disabled) emit('update:modelValue', color);
}
function selectPreset(value: string) {
  const color = normalize(value);
  if (!color || props.disabled) return;
  candidate.value = color;
  emit('update:modelValue', color);
}
</script>

<template>
  <PopoverRoot v-model:open="open">
    <PopoverTrigger as-child>
      <button type="button" class="base-color-trigger" :disabled="disabled" :aria-label="`${label}: ${safeValue}`">
        <span class="base-color-swatch" :style="{ backgroundColor: safeValue }" aria-hidden="true" />
        <span class="base-color-trigger-label">{{ label }}</span>
        <code>{{ safeValue }}</code>
      </button>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent class="base-popover-content base-color-popover" :aria-label="t('appearanceColorPicker')" :side-offset="6" :collision-padding="8">
        <h3>{{ label }}</h3>
        <div v-if="presets.length" class="base-color-presets" role="group" :aria-label="t('colorPreset')">
          <button
            v-for="color in presets"
            :key="color"
            type="button"
            class="base-color-preset"
            :style="{ '--preset-color': normalize(color) ?? safeValue }"
            :aria-label="`${t('colorPreset')} ${normalize(color) ?? color}`"
            :aria-pressed="safeValue === normalize(color)"
            :title="normalize(color) ?? color"
            :disabled="disabled || !normalize(color)"
            @click="selectPreset(color)"
          >
            <span aria-hidden="true" />
          </button>
        </div>
        <label class="base-color-hex-field">
          <span>{{ t('colorHexLabel') }}</span>
          <input
            class="base-input"
            type="text"
            spellcheck="false"
            autocomplete="off"
            maxlength="7"
            placeholder="#RRGGBB"
            :value="candidate"
            :aria-label="t('colorHexLabel')"
            :aria-invalid="!valid"
            :disabled="disabled"
            @input="updateCandidate(($event.target as HTMLInputElement).value)"
          />
        </label>
        <p v-if="!valid" class="base-color-error" role="alert">{{ t('colorHexInvalid') }}</p>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
