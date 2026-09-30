<script setup lang="ts">
import { computed, useId } from "vue";
defineOptions({ inheritAttrs: false });
const props = defineProps<{ modelValue: string; label: string; id?: string; error?: string; disabled?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const generatedId = useId();
const inputId = computed(() => props.id ?? generatedId);
function onInput(event: Event) {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="base-field">
    <label :for="inputId">{{ label }}</label>
    <input v-bind="$attrs" :id="inputId" class="base-input" :value="modelValue" :disabled="disabled"
      :aria-invalid="!!error" :aria-describedby="error ? `${inputId}-error` : undefined" @input="onInput" />
    <p v-if="error" :id="`${inputId}-error`" class="base-field-error">{{ error }}</p>
  </div>
</template>
