<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import type { AppError } from "../../../../contracts/v1/AppError";
import type { RemoteFileEntry } from "../../../../contracts/v1/RemoteFileEntry";
import type { createSftpApi } from "../../ipc/sftp";
import { mapError } from "../../errors/mapper";
import { presentError } from "../../errors/presenter";
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { renderMarkdownPreview } from "../../files/markdown";
import { resolveMarkdownFileLink } from "../../files/path";
import BaseAlert from "../base/BaseAlert.vue";
import BaseButton from "../base/BaseButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import BaseInput from "../base/BaseInput.vue";

const props = defineProps<{
  api: ReturnType<typeof createSftpApi>;
  connectionId: string;
  entry: RemoteFileEntry;
  open: boolean;
}>();
const emit = defineEmits<{ close: []; saved: []; openLinkedFile: [entry: RemoteFileEntry] }>();
const t = messages(filesMessages);
const loading = ref(false);
const loaded = ref(false);
const saving = ref(false);
const mode = ref<'view' | 'edit'>('view');
const markdownPanel = ref<'preview' | 'source'>('preview');
const content = ref('');
const draft = ref('');
const revision = ref('');
const error = shallowRef<AppError | null>(null);
const linkNotice = ref('');
const pendingLinkPath = ref<string | null>(null);
const linkConfirmationOpen = ref(false);
const openingLinkedFile = ref(false);
const elevationRequired = ref(false);
const sudoDialog = ref(false);
const sudoPassword = ref('');
const sudoError = shallowRef<AppError | null>(null);
const exitAfterDiscard = ref<'view' | 'close' | null>(null);
const dirty = computed(() => mode.value === 'edit' && draft.value !== content.value);
const isMarkdown = computed(() => props.entry.name.toLocaleLowerCase().endsWith('.md'));
const markdownSource = computed(() => mode.value === 'edit' ? draft.value : content.value);
const markdownPreview = computed(() => renderMarkdownPreview(
  markdownSource.value,
  t('markdownImagePreviewBlocked'),
  t('markdownTaskChecked'),
  t('markdownTaskUnchecked'),
));
let generation = 0;

async function load() {
  const token = ++generation;
  loading.value = true; loaded.value = false; error.value = null; linkNotice.value = ''; content.value = ''; draft.value = ''; revision.value = ''; mode.value = 'view'; markdownPanel.value = 'preview'; elevationRequired.value = false; sudoDialog.value = false; sudoPassword.value = ''; sudoError.value = null; exitAfterDiscard.value = null;
  try {
    const result = await props.api.readText({ connectionId: props.connectionId, path: props.entry.path });
    if (token !== generation) return;
    content.value = result.content; draft.value = result.content; revision.value = result.revision;
    loaded.value = true;
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

function enterEdit() { draft.value = content.value; mode.value = 'edit'; markdownPanel.value = 'source'; }
function requestView() {
  if (dirty.value) { exitAfterDiscard.value = 'view'; return; }
  mode.value = 'view'; markdownPanel.value = 'preview'; elevationRequired.value = false;
}
function discard() {
  const destination = exitAfterDiscard.value;
  draft.value = content.value; mode.value = 'view'; markdownPanel.value = 'preview'; elevationRequired.value = false; error.value = null; exitAfterDiscard.value = null;
  if (destination === 'close') emit('close');
}

function cancelSaveAndRestore() {
  draft.value = content.value; mode.value = 'view'; markdownPanel.value = 'preview'; elevationRequired.value = false; error.value = null;
}

function continueWithSudo() {
  sudoPassword.value = ''; sudoError.value = null; sudoDialog.value = true;
}

function handleMarkdownLinkClick(event: MouseEvent) {
  const target = event.target;
  const anchor = target instanceof Element ? target.closest<HTMLAnchorElement>('a[href]') : null;
  if (!anchor) return;
  event.preventDefault();

  const href = anchor.getAttribute('href') ?? '';
  if (href.trim().startsWith('#')) return;
  if (mode.value !== 'view') { linkNotice.value = t('markdownLinkEditMode'); return; }

  const path = resolveMarkdownFileLink(props.entry.path, href);
  if (!path) { linkNotice.value = t('markdownLinkUnsupported'); return; }
  linkNotice.value = '';
  pendingLinkPath.value = path;
  linkConfirmationOpen.value = true;
}

function cancelLinkedFilePreview() {
  if (openingLinkedFile.value) return;
  linkConfirmationOpen.value = false;
  pendingLinkPath.value = null;
}

async function previewLinkedFile() {
  const path = pendingLinkPath.value;
  if (!path || openingLinkedFile.value) return;
  linkConfirmationOpen.value = false;
  openingLinkedFile.value = true;
  linkNotice.value = '';
  try {
    const entry = await props.api.stat({ connectionId: props.connectionId, path, followSymlink: false });
    if (entry.fileType !== 'file' || entry.isSymlink) {
      linkNotice.value = t('markdownLinkNotPreviewable', { path });
      return;
    }
    pendingLinkPath.value = null;
    emit('openLinkedFile', entry);
  } catch (reason) {
    const mapped = mapError(reason);
    linkNotice.value = mapped.code === 'PATH_NOT_FOUND'
      ? t('markdownLinkNotFound', { path })
      : presentError(mapped).message;
  } finally {
    pendingLinkPath.value = null;
    openingLinkedFile.value = false;
  }
}

function closeSudoDialog() {
  if (saving.value) return;
  sudoDialog.value = false; sudoPassword.value = ''; sudoError.value = null;
}

function commitSavedContent(nextRevision: string) {
  content.value = draft.value; revision.value = nextRevision; mode.value = 'view'; markdownPanel.value = 'preview'; elevationRequired.value = false; error.value = null; sudoError.value = null; sudoDialog.value = false; sudoPassword.value = '';
  emit('saved');
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
    commitSavedContent(result.revision);
  } catch (reason) {
    const mapped = mapError(reason);
    if (mapped.code === 'PERMISSION_DENIED') { elevationRequired.value = true; error.value = mapped; }
    else error.value = mapped;
  }
  finally { saving.value = false; }
}

async function saveWithSudo() {
  if (!dirty.value || saving.value || !sudoPassword.value) return;
  const password = sudoPassword.value;
  sudoPassword.value = '';
  sudoError.value = null; saving.value = true;
  try {
    const result = await props.api.writeTextWithSudo({
      connectionId: props.connectionId,
      path: props.entry.path,
      content: draft.value,
      expectedRevision: revision.value,
      password,
    });
    commitSavedContent(result.revision);
  } catch (reason) { sudoError.value = mapError(reason); }
  finally { saving.value = false; }
}
</script>

<template>
  <BaseDialog :open="open" size="large" :title="entry.name" :busy="saving || openingLinkedFile" panel-class="file-editor-dialog" initial-focus=".file-editor-mode" @close="requestClose">
    <p class="file-editor-path"><code>{{ entry.path }}</code></p>
    <BaseAlert v-if="error && !elevationRequired" class="file-editor-error">{{ presentError(error).message }}</BaseAlert>
    <BaseAlert v-if="linkNotice" class="file-editor-error">{{ linkNotice }}</BaseAlert>
    <div v-if="loading" class="file-editor-loading" role="status">{{ t('readingTextFile') }}</div>
    <template v-else-if="loaded">
      <div v-if="isMarkdown" class="file-markdown-switcher" role="group" :aria-label="t('markdownPreview')">
        <button type="button" :aria-pressed="markdownPanel === 'preview'" @click="markdownPanel = 'preview'">{{ t('markdownPreview') }}</button>
        <button type="button" :aria-pressed="markdownPanel === 'source'" @click="markdownPanel = 'source'">{{ t('markdownSource') }}</button>
      </div>
      <div v-if="isMarkdown && markdownPanel === 'preview'" class="file-editor-markdown" v-html="markdownPreview" @click="handleMarkdownLinkClick" />
      <pre v-else-if="mode === 'view'" class="file-editor-content">{{ content }}</pre>
      <textarea v-else v-model="draft" class="file-editor-textarea" :aria-label="t('fileContent')" :disabled="saving" spellcheck="false" />
      <div v-if="elevationRequired" class="file-editor-elevation" role="alert">
        <strong>{{ t('sudoSaveTitle') }}</strong>
        <p>{{ presentError(error!).message }}</p>
        <p>{{ t('sudoSaveDescription') }}</p>
      </div>
      <div v-if="exitAfterDiscard" class="file-editor-discard" role="alertdialog" :aria-label="t('unsavedChanges')">
        <span>{{ t('unsavedChanges') }}</span>
        <BaseButton :disabled="saving" @click="exitAfterDiscard = null">{{ t('continueEditing') }}</BaseButton>
        <BaseButton :disabled="saving" @click="discard">{{ t(exitAfterDiscard === 'close' ? 'discardAndClose' : 'discardAndView') }}</BaseButton>
      </div>
    </template>
    <template #footer>
      <BaseButton v-if="error && !loaded" :disabled="loading || saving" @click="load">{{ t('retryRead') }}</BaseButton>
      <template v-if="mode === 'view'">
        <BaseButton data-dialog-cancel :disabled="loading || saving" @click="requestClose">{{ t('cancel') }}</BaseButton>
        <BaseButton v-if="!error" class="file-editor-mode" variant="primary" :disabled="loading || saving" @click="enterEdit">{{ t('editMode') }}</BaseButton>
      </template>
      <template v-else-if="elevationRequired">
        <BaseButton data-dialog-cancel :disabled="saving" @click="cancelSaveAndRestore">{{ t('cancelSave') }}</BaseButton>
        <BaseButton variant="primary" :disabled="saving || !dirty" @click="continueWithSudo">{{ t('continueWithSudo') }}</BaseButton>
      </template>
      <template v-else-if="!exitAfterDiscard">
        <BaseButton data-dialog-cancel :disabled="saving" @click="requestView">{{ t('viewMode') }}</BaseButton>
        <BaseButton variant="primary" :disabled="saving || !dirty" @click="save">{{ saving ? t('saving') : t('save') }}</BaseButton>
      </template>
    </template>
  </BaseDialog>
  <BaseDialog :open="linkConfirmationOpen" size="compact" :title="t('markdownLinkConfirmTitle')" :busy="openingLinkedFile" panel-class="file-editor-link-dialog" initial-focus=".markdown-link-cancel" @close="cancelLinkedFilePreview">
    <p>{{ t('markdownLinkConfirmBody', { path: pendingLinkPath ?? '' }) }}</p>
    <template #footer>
      <BaseButton class="markdown-link-cancel" :disabled="openingLinkedFile" @click="cancelLinkedFilePreview">{{ t('cancel') }}</BaseButton>
      <BaseButton variant="primary" :disabled="openingLinkedFile || !pendingLinkPath" :loading="openingLinkedFile" @click="previewLinkedFile">{{ t('markdownLinkPreview') }}</BaseButton>
    </template>
  </BaseDialog>
  <BaseDialog :open="sudoDialog" size="compact" :title="t('sudoSaveTitle')" :busy="saving" panel-class="file-editor-sudo-dialog" initial-focus=".file-editor-sudo-password" @close="closeSudoDialog">
    <p>{{ t('sudoSaveDescription') }}</p>
    <BaseAlert v-if="sudoError" class="file-editor-error">{{ presentError(sudoError).message }}</BaseAlert>
    <BaseInput v-model="sudoPassword" class="file-editor-sudo-password" :label="t('sudoPasswordLabel')" type="password" autocomplete="off" :disabled="saving" @keydown.enter.prevent="saveWithSudo" />
    <p class="file-editor-sudo-hint">{{ t('sudoPasswordHint') }}</p>
    <template #footer>
      <BaseButton data-dialog-cancel :disabled="saving" @click="closeSudoDialog">{{ t('cancel') }}</BaseButton>
      <BaseButton variant="primary" :disabled="saving || !sudoPassword" :loading="saving" @click="saveWithSudo">{{ t('sudoSaving') }}</BaseButton>
    </template>
  </BaseDialog>
</template>
