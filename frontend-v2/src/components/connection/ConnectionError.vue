<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { AppError } from "../../../../contracts/v1/AppError";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
const props = defineProps<{ error: AppError }>();
const shown = ref(false);
const presentation = computed(() => presentError(props.error));
watch(() => props.error, () => { shown.value = false; });
</script>
<template>
  <div class="connection-error" role="alert">
    <p>{{ presentation.message }}</p>
    <p v-if="presentation.stage" class="connection-muted">阶段：{{ presentation.stage }}</p>
    <BaseButton :aria-expanded="shown" @click="shown = !shown">{{ shown ? '收起诊断' : '查看诊断' }}</BaseButton>
    <dl v-if="shown" class="connection-diagnostics"><dt>错误代码</dt><dd>{{ error.code }}</dd><dt>Request ID</dt><dd>{{ error.requestId ?? '—' }}</dd><dt>阶段</dt><dd>{{ error.stage ?? '—' }}</dd></dl>
  </div>
</template>
