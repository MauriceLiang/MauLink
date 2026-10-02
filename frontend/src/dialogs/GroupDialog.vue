<script setup lang="ts">
import BaseAlertDialog from "../components/base/BaseAlertDialog.vue";
import { nextTick, ref, useId, watch } from "vue";
import type { Group } from "../../../contracts/v1/Group";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerStore } from "../stores/servers";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseInput from "../components/base/BaseInput.vue";
const props = withDefaults(defineProps<{ open: boolean; store: ServerStore; language?: Language }>(), {});
const emit = defineEmits<{ close: []; saved: [message: string] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const groups = props.store.groups;
const nameId = useId();
const name = ref("");
const editing = ref<Group | null>(null);
const deleting = ref<Group | null>(null);
const busy = ref(false);
const error = ref("");
const conflict = ref(false);
function reset() {
  name.value = ""; editing.value = null; deleting.value = null; error.value = ""; conflict.value = false;
  void nextTick(() => { if (props.open) document.getElementById(nameId)?.focus(); });
}
watch(() => props.open, open => { if (open) reset(); });
function showError(reason: unknown) {
  const failure = mapError(reason);
  error.value = presentError(failure, props.language).message;
  conflict.value = failure.code === "REVISION_CONFLICT";
}
async function save() {
  if (busy.value) return;
  const value = name.value.trim();
  if (!value || Array.from(value).length > 64 || /[\u0000-\u001f\u007f]/u.test(value)) { error.value = t("groupRequired"); return; }
  busy.value = true;
  error.value = "";
  try {
    if (editing.value) await props.store.updateGroup({ groupId: editing.value.id, update: { name: value, sortOrder: editing.value.sortOrder, expectedRevision: editing.value.revision } });
    else await props.store.createGroup({ name: value, sortOrder: Math.max(-1, ...groups.value.map(group => group.sortOrder)) + 1 });
    reset();
    emit("saved", t("groupSaved"));
  } catch (reason) { showError(reason); }
  finally { busy.value = false; }
}
async function remove() {
  if (!deleting.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    await props.store.removeGroup({ id: deleting.value.id, expectedRevision: deleting.value.revision });
    reset();
    emit("saved", t("groupRemoved"));
  } catch (reason) { showError(reason); }
  finally { busy.value = false; }
}
async function reload() {
  busy.value = true;
  if (await props.store.load()) reset();
  else error.value = props.store.error.value?.message ?? t("retry");
  busy.value = false;
}
</script>

<template>
  <component :is="deleting ? BaseAlertDialog : BaseDialog" :initial-focus="deleting ? '[data-dialog-cancel]' : 'input'" :open="open" :title="t('groups')" :busy="busy" :close-label="t('cancel')" panel-class="server-group-dialog" @close="$emit('close')">
    <template v-if="deleting"><p>{{ deleting.name }}</p><p>{{ t('groupDeleteNote') }}</p></template>
    <template v-else>
      <ul class="server-group-manager"><li v-for="group in groups" :key="group.id"><span>{{ group.name }}</span>
        <BaseButton :disabled="busy" :aria-label="`${t('rename')} ${group.name}`" @click="editing = { ...group }; name = group.name; error = ''; conflict = false">{{ t('rename') }}</BaseButton>
        <BaseButton :disabled="busy" :aria-label="`${t('deleteGroup')} ${group.name}`" @click="deleting = { ...group }; error = ''; conflict = false">{{ t('deleteGroup') }}</BaseButton>
      </li></ul>
      <form class="server-group-form" @submit.prevent="save"><BaseInput :id="nameId" v-model="name" :label="t('groupName')" :disabled="busy" maxlength="64" />
        <BaseButton v-if="editing" :disabled="busy" @click="reset">{{ t('cancel') }}</BaseButton><BaseButton type="submit" variant="primary" :loading="busy">{{ t(editing ? 'save' : 'newGroup') }}</BaseButton>
      </form>
    </template>
    <p v-if="error" class="server-form-error" role="alert">{{ error }}</p>
    <BaseButton v-if="conflict" :disabled="busy" @click="reload">{{ t('reload') }}</BaseButton>
    <template #footer><BaseButton data-dialog-cancel :disabled="busy" @click="deleting ? reset() : $emit('close')">{{ t('cancel') }}</BaseButton><BaseButton v-if="deleting" variant="danger" :loading="busy" @click="remove">{{ t('deleteGroup') }}</BaseButton></template>
  </component>
</template>
