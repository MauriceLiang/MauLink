<script setup lang="ts">
import { messages } from "../../i18n/locale";
import { connectionMessages } from "../../i18n/connection";
import { computed, nextTick, ref, watch } from "vue";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { ConnectionStore } from "../../stores/connections";
import { isFinished } from "../../stores/connections";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import ConnectionError from "./ConnectionError.vue";
const t = messages(connectionMessages);
const props = defineProps<{ server: ServerProfile; store: ConnectionStore; readOnly: boolean; connectLabel?: string }>();
const snapshot = computed(() => props.store.snapshots.value[props.server.id]);
const busy = computed(() => !!props.store.busy.value[props.server.id]);
const transportError = computed(() => props.store.errors.value[props.server.id]);
const error = computed(() => transportError.value ?? snapshot.value?.error);
const active = computed(() => snapshot.value && !isFinished(snapshot.value));
const retryable = computed(() => error.value && presentError(error.value).retryable && !active.value);
const errorOpen = ref(false);
const panel = ref<HTMLElement | null>(null);
watch(error, value => { errorOpen.value = !!value; }, { immediate: true });

function retry() {
  errorOpen.value = false;
  void props.store.start(props.server);
}

async function restoreFocus() {
  await nextTick();
  if (document.activeElement === document.body && !document.querySelector('[role="dialog"], [role="alertdialog"]')) {
    panel.value?.querySelector<HTMLButtonElement>('.connection-actions button:not(:disabled)')?.focus({ preventScroll: true });
  }
}

</script>
<template>
  <section ref="panel" class="connection-panel" :aria-label="t('connection', {name: server.name})" :aria-busy="busy">
    <BaseDialog :open="errorOpen && !!error && !store.challenge.value" :title="t('connectionFailed')" panel-class="connection-dialog" :close-label="t('close')" @close="errorOpen = false" @closed="restoreFocus">
      <ConnectionError v-if="error" :error="error" />
      <template #footer>
        <BaseButton @click="errorOpen = false">{{ t('close') }}</BaseButton>
        <BaseButton v-if="retryable" variant="primary" :disabled="readOnly || !!store.challenge.value" :loading="busy" @click="retry">{{ t('retryConnection') }}</BaseButton>
      </template>
    </BaseDialog>
    <div class="connection-actions">
      <BaseButton v-if="!active && (!error || retryable)" variant="primary" :disabled="readOnly || !!store.challenge.value" :loading="busy" @click="store.start(server)">{{ retryable ? t('retryConnection') : connectLabel ?? t('connect') }}</BaseButton>
      <BaseButton v-if="active && snapshot?.state !== 'ready'" :disabled="busy" @click="store.cancel(server.id)">{{ t('cancelConnection') }}</BaseButton>
      <BaseButton v-if="snapshot?.state === 'ready'" :disabled="busy" @click="store.disconnect(server.id)">{{ t('disconnect') }}</BaseButton>
      <BaseButton v-if="transportError && active" :disabled="busy" @click="store.refresh(server.id)">{{ t('refreshConnectionStatus') }}</BaseButton>
      <BaseButton v-if="error && !active && !retryable" :disabled="busy" @click="store.dismiss(server.id)">{{ t('dismissError') }}</BaseButton>
      <slot />
    </div>
  </section>
</template>
