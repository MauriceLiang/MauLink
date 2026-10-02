<script setup lang="ts">
import BaseAlert from "../base/BaseAlert.vue";
import { messages } from "../../i18n/locale";
import { connectionMessages } from "../../i18n/connection";
import { computed, ref, watch } from "vue";
import type { AppError } from "../../../../contracts/v1/AppError";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
const t = messages(connectionMessages);
const props = defineProps<{ error: AppError }>();
const shown = ref(false);
const presentation = computed(() => presentError(props.error));
watch(() => props.error, () => { shown.value = false; });
</script>
<template>
  <BaseAlert class="connection-error">
    <p>{{ presentation.message }}</p>
    <p v-if="presentation.stage" class="connection-muted">{{ t('stage') }}: {{ presentation.stage }}</p>
    <BaseButton :aria-expanded="shown" @click="shown = !shown">{{ shown ? t('hideDiagnostics') : t('showDiagnostics') }}</BaseButton>
    <dl v-if="shown" class="connection-diagnostics"><dt>{{ t('errorCode') }}</dt><dd>{{ error.code }}</dd><dt>Request ID</dt><dd>{{ error.requestId ?? '—' }}</dd><dt>{{ t('stage') }}</dt><dd>{{ error.stage ?? '—' }}</dd></dl>
  </BaseAlert>
</template>
