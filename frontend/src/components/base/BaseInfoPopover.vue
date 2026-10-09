<script setup lang="ts">
import BaseIcon from "./BaseIcon.vue";
import BasePopover from "./BasePopover.vue";

withDefaults(defineProps<{
  title: string;
  description: string;
  triggerLabel: string;
  side?: "top" | "right" | "bottom" | "left";
  disabled?: boolean;
}>(), { side: "right", disabled: false });
</script>

<template>
  <BasePopover :title="title" :side="side">
    <template #trigger>
      <button type="button" class="base-info-trigger" :aria-label="triggerLabel" :disabled="disabled">
        <BaseIcon name="info" />
      </button>
    </template>
    <p class="base-info-popover-text">{{ description }}</p>
  </BasePopover>
</template>
