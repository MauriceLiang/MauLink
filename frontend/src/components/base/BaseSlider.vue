<script setup lang="ts">
import { computed } from 'vue';
import { SliderRange, SliderRoot, SliderThumb, SliderTrack } from 'reka-ui';

const props = withDefaults(defineProps<{
  modelValue: number;
  label: string;
  min: number;
  max: number;
  step?: number;
  unit?: '%' | 'px' | '';
  disabled?: boolean;
  id?: string;
}>(), { step: 1, unit: '', disabled: false });
const emit = defineEmits<{ 'update:modelValue': [value: number] }>();

const safeMin = computed(() => Number.isFinite(props.min) ? props.min : 0);
const safeMax = computed(() => Number.isFinite(props.max) ? Math.max(safeMin.value, props.max) : safeMin.value);
const safeStep = computed(() => Number.isFinite(props.step) && props.step > 0 ? props.step : 1);
const value = computed(() => Number.isFinite(props.modelValue)
  ? Math.min(safeMax.value, Math.max(safeMin.value, props.modelValue))
  : safeMin.value);
const formattedValue = computed(() => `${value.value}${props.unit === 'px' ? ' px' : props.unit}`);

function update(values?: number[]) {
  if (props.disabled || !values || !Number.isFinite(values[0])) return;
  const clamped = Math.min(safeMax.value, Math.max(safeMin.value, values[0]!));
  const stepped = safeMin.value + Math.round((clamped - safeMin.value) / safeStep.value) * safeStep.value;
  const precision = Math.max(0, (String(safeStep.value).split('.')[1] ?? '').length);
  emit('update:modelValue', Number(Math.min(safeMax.value, Math.max(safeMin.value, stepped)).toFixed(precision)));
}
</script>

<template>
  <div class="base-slider-field" :data-disabled="disabled || undefined">
    <div class="base-slider-heading">
      <label :for="id">{{ label }}</label>
      <output class="base-slider-value" :for="id">{{ formattedValue }}</output>
    </div>
    <SliderRoot
      class="base-slider"
      :model-value="[value]"
      :min="safeMin"
      :max="safeMax"
      :step="safeStep"
      :disabled="disabled"
      @update:model-value="update"
    >
      <SliderTrack class="base-slider-track">
        <SliderRange class="base-slider-range" />
      </SliderTrack>
      <SliderThumb :id="id" class="base-slider-thumb" :aria-label="label" />
    </SliderRoot>
  </div>
</template>
