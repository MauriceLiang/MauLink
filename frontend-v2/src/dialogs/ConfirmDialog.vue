<script setup lang="ts">
import { ref, watch } from "vue";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerStore } from "../stores/servers";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
const props = withDefaults(defineProps<{ server: ServerProfile | null; store: ServerStore; language?: Language }>(), { language: "zh-CN" });
const emit = defineEmits<{ close: []; removed: [message: string] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const busy = ref(false);
const error = ref("");
const conflict = ref(false);
watch(() => props.server, () => { error.value = ""; conflict.value = false; });
async function remove() {
  if (!props.server || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await props.store.remove({ serverId: props.server.id, expectedRevision: props.server.revision, removeCredentials: true });
    emit("removed", result.credentialCleanupPending ? t("cleanup") : t("removed"));
    emit("close");
  } catch (reason) {
    const failure = mapError(reason);
    error.value = presentError(failure, props.language).message;
    conflict.value = failure.code === "REVISION_CONFLICT";
  } finally { busy.value = false; }
}
async function reload() {
  busy.value = true;
  if (await props.store.load()) emit("close");
  else error.value = props.store.error.value?.message ?? t("retry");
  busy.value = false;
}
</script>

<template>
  <BaseDialog :open="!!server" :title="t('deleteTitle')" :busy="busy" :close-label="t('cancel')" @close="$emit('close')">
    <p>{{ server?.name }}</p><p>{{ t('deleteNote') }}</p>
    <p v-if="error" role="alert" class="server-form-error">{{ error }}</p>
    <BaseButton v-if="conflict" :disabled="busy" @click="reload">{{ t('reload') }}</BaseButton>
    <template #footer><BaseButton :disabled="busy" @click="$emit('close')">{{ t('cancel') }}</BaseButton><BaseButton variant="danger" :loading="busy" @click="remove">{{ t('remove') }}</BaseButton></template>
  </BaseDialog>
</template>
