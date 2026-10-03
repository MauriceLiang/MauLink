<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import type { AppError } from "../../../../contracts/v1/AppError";
import type { RemoteFileEntry } from "../../../../contracts/v1/RemoteFileEntry";
import type { createSftpApi } from "../../ipc/sftp";
import { mapError } from "../../errors/mapper";
import { presentError } from "../../errors/presenter";
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import BaseAlert from "../base/BaseAlert.vue";
import BaseButton from "../base/BaseButton.vue";
import BaseDialog from "../base/BaseDialog.vue";

const props = defineProps<{
  api: ReturnType<typeof createSftpApi>;
  connectionId: string;
  entry: RemoteFileEntry;
  open: boolean;
}>();
const emit = defineEmits<{ close: []; saved: [] }>();
const t = messages(filesMessages);
const loading = ref(false);
const saving = ref(false);
const mode = ref<'view' | 'edit'>('view');
const content = ref('');
const draft = ref('');
const revision = ref('');
const error = shallowRef<AppError | null>(null);
const exitAfterDiscard = ref<'view' | 'close' | null>(null);
const dirty = computed(() => mode.value === 'edit' && draft.value !== content.value);
let generation = 0;

async function load() {
  const token = ++generation;
  loading.value = true; error.value = null; content.value = ''; draft.value = ''; revision.value = ''; mode.value = 'view'; exitAfterDiscard.value = null;
  try {
    const result = await props.api.readText({ connectionId: props.connectionId, path: props.entry.path });
    if (token !== generation) return;
    content.value = result.content; draft.value = result.content; revision.value = result.revision;
  } catch (reason) {
    if (token === generation) error.value = mapError(reason);
  } finally {
    if (token === generation) loading.value = false;
  }
}

watch([() => props.open, () => props.entry.path], ([open]) => { if (open) void load(); else ++generation; }, { immediate: true });
onBeforeUnmount(() => { ++generation; });

function requestClose() {
  if (saving.value) return;
  if (dirty.value) { exitAfterDiscard.value = 'close'; return; }
  emit('close');
}

function enterEdit() { draft.value = content.value; mode.value = 'edit'; }
function requestView() {
  if (dirty.value) { exitAfterDiscard.value = 'view'; return; }
  mode.value = 'view';
}
function discard() {
  const destination = exitAfterDiscard.value;
  draft.value = content.value; mode.value = 'view'; exitAfterDiscard.value = null;
  if (destination === 'close') emit('close');
}

async function save() {
  if (!dirty.value || saving.value) return;
  saving.value = true; error.value = null;
  try {
    const result = await props.api.writeText({
      connectionId: props.connectionId,
      path: props.entry.path,
      content: draft.value,
      expectedRevision: revision.value,
    });
    content.value = draft.value; revision.value = result.revision; mode.value = 'view';
    emit('saved');
  } catch (reason) { error.value = mapError(reason); }
  finally { saving.value = false; }
}
</script>

<template>
  <BaseDialog :open="open" :title="entry.name" :busy="saving" panel-class="file-editor-dialog" initial-focus=".file-editor-mode" @close="requestClose">
    <p class="file-editor-path"><code>{{ entry.path }}</code></p>
    <BaseAlert v-if="error" class="file-editor-error">{{ presentError(error).message }}</BaseAlert>
    <div v-if="loading" class="file-editor-loading" role="status">{{ t('readingTextFile') }}</div>
    <template v-else-if="!error">
      <pre v-if="mode === 'view'" class="file-editor-content">{{ content }}</pre>
      <textarea v-else v-model="draft" class="file-editor-textarea" :aria-label="t('fileContent')" :disabled="saving" spellcheck="false" />
      <div v-if="exitAfterDiscard" class="file-editor-discard" role="alertdialog" :aria-label="t('unsavedChanges')">
        <span>{{ t('unsavedChanges') }}</span>
        <BaseButton :disabled="saving" @click="exitAfterDiscard = null">{{ t('continueEditing') }}</BaseButton>
        <BaseButton :disabled="saving" @click="discard">{{ t(exitAfterDiscard === 'close' ? 'discardAndClose' : 'discardAndView') }}</BaseButton>
      </div>
    </template>
    <template #footer>
      <BaseButton v-if="error" :disabled="loading || saving" @click="load">{{ t('retryRead') }}</BaseButton>
      <template v-if="mode === 'view'">
        <BaseButton data-dialog-cancel :disabled="loading || saving" @click="requestClose">{{ t('cancel') }}</BaseButton>
        <BaseButton v-if="!error" class="file-editor-mode" variant="primary" :disabled="loading || saving" @click="enterEdit">{{ t('editMode') }}</BaseButton>
      </template>
      <template v-else-if="!exitAfterDiscard">
        <BaseButton data-dialog-cancel :disabled="saving" @click="requestView">{{ t('viewMode') }}</BaseButton>
        <BaseButton variant="primary" :disabled="saving || !dirty" @click="save">{{ saving ? t('saving') : t('save') }}</BaseButton>
      </template>
    </template>
  </BaseDialog>
</template>
