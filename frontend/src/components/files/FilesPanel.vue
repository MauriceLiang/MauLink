<script setup lang="ts">
import BaseAlert from "../base/BaseAlert.vue";
import BaseTooltip from "../base/BaseTooltip.vue";
import BaseAlertDialog from "../base/BaseAlertDialog.vue";
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import type { RemoteFileEntry } from "../../../../contracts/v1/RemoteFileEntry";
import type { createSftpApi } from "../../ipc/sftp";
import type { TransferStore } from "../../stores/transfers";
import { createFilesStore } from "../../stores/files";
import { breadcrumbs, formatSize, modifiedTime, parentRemotePath, validBasename } from "../../files/path";
import { mapError } from "../../errors/mapper";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import FileMenu from "./FileMenu.vue";
import TransferPanel from "./TransferPanel.vue";
const t = messages(filesMessages);
const props = defineProps<{ api: ReturnType<typeof createSftpApi>; transfers: TransferStore; connectionId: string; visible: boolean; ready: boolean }>();
const files = createFilesStore(props.api, props.connectionId);
const { entries, path, cursor, page, pending, mutating, needsReload, error, cleanupError } = files;
const location = ref(''); const selectedPath = ref<string | null>(null); const pathInput = ref<HTMLInputElement>();
const selected = computed(() => entries.value.find(entry => entry.path === selectedPath.value));
const unavailable = computed(() => !props.ready || pending.value || mutating.value || needsReload.value);
const operation = ref<'mkdir' | 'rename' | 'delete' | null>(null);
const target = shallowRef<RemoteFileEntry | null>(null); const name = ref(''); const notice = ref('');
const uploadRefresh = ref(false); const completedUploads = new Set<string>();
watch(path, value => { location.value = value; });
watch(entries, () => { selectedPath.value = null; });
watch(() => props.ready, ready => { if (!ready) files.suspend(); else if (props.visible && needsReload.value) void files.load(path.value || '.'); });
watch(() => props.visible, visible => {
  if (visible && props.ready && (!path.value || needsReload.value)) void files.load(path.value || '.');
  if (visible) void props.transfers.load(props.connectionId);
}, { immediate: true });
watch(props.transfers.snapshots, tasks => {
  for (const task of tasks) {
    if (task.connectionId === props.connectionId && task.direction === 'upload' && task.state === 'completed' && !completedUploads.has(task.transferId)) {
      completedUploads.add(task.transferId); uploadRefresh.value = true;
    }
  }
});
watch([uploadRefresh, () => props.visible, pending, mutating], () => {
  if (uploadRefresh.value && props.visible && props.ready && !pending.value && !mutating.value) { uploadRefresh.value = false; void files.load(path.value || '.'); }
});
async function navigate(destination: string) { if (!props.ready || mutating.value) return; notice.value = ''; await files.load(destination); }
function edit(value: 'mkdir' | 'rename' | 'delete', entry?: RemoteFileEntry) {
  if (unavailable.value || (value !== 'mkdir' && !entry)) return;
  error.value = null; operation.value = value; target.value = entry ? { ...entry } : null; name.value = value === 'rename' ? entry!.name : '';
}
async function submit() {
  if (!operation.value || unavailable.value || (operation.value !== 'delete' && !validBasename(name.value))) return;
  const success = operation.value === 'mkdir' ? await files.mkdir(name.value) : operation.value === 'rename' ? await files.rename(target.value!, name.value) : await files.remove(target.value!);
  if (success) { operation.value = null; notice.value = t('fileOperationCompleted'); await files.load(); await nextTick(); pathInput.value?.focus(); }
}
async function transfer(direction: 'upload' | 'download', entry?: RemoteFileEntry) {
  if (unavailable.value || (direction === 'download' && !entry)) return;
  const directory = path.value;
  await props.transfers.start(direction, props.connectionId, direction === 'upload' ? directory : entry!.path,
    () => props.visible && !unavailable.value && path.value === directory);
}
async function action(value: 'download' | 'rename' | 'delete' | 'copy', entry: RemoteFileEntry) {
  if (unavailable.value) return;
  if (value === 'download') await transfer('download', entry);
  else if (value === 'copy') {
    try { await navigator.clipboard.writeText(entry.path); notice.value = t('pathCopied'); }
    catch (reason) { error.value = mapError(reason); }
  } else edit(value, entry);
}
const downloadable = (entry: RemoteFileEntry) => entry.fileType === 'file' || entry.isSymlink || entry.fileType === 'symlink';
onBeforeUnmount(files.dispose);
</script>
<template>
  <div class="files-workspace">
    <section class="files-browser" :aria-label="t('remoteFiles')">
      <div class="files-heading"><h2>{{ t('remoteFiles') }}</h2><span>SFTP</span><BaseButton :disabled="!ready || pending || mutating" @click="navigate(path || '.')">{{ t('refreshDirectory') }}</BaseButton></div>
      <form class="files-location" @submit.prevent="navigate(location)"><BaseTooltip :label="t('parentDirectory')"><BaseButton :disabled="!ready || pending || mutating || !path || path === '/'" :aria-label="t('parentDirectory')" @click="navigate(parentRemotePath(path))">↑</BaseButton></BaseTooltip><input ref="pathInput" v-model="location" :aria-label="t('remotePath')" :disabled="!ready || mutating" :placeholder="t('remotePathEGHome')" /><BaseButton type="submit" :disabled="!ready || pending || mutating">{{ t('go') }}</BaseButton></form>
      <nav class="files-breadcrumb" :aria-label="t('directoryNavigation')"><button v-for="part in breadcrumbs(path)" :key="part.path" :aria-current="part.path === path ? 'location' : undefined" :disabled="!ready || pending || mutating" @click="navigate(part.path)">{{ part.name }}</button></nav>
      <div class="files-toolbar"><BaseButton :disabled="unavailable || transfers.starting.value[connectionId]" @click="transfer('upload')">{{ t('uploadFile') }}</BaseButton><BaseButton :disabled="unavailable" @click="edit('mkdir')">{{ t('newFolder') }}</BaseButton><BaseButton :disabled="unavailable || !selected || !downloadable(selected) || transfers.starting.value[connectionId]" @click="selected && transfer('download', selected)">{{ t('downloadFile') }}</BaseButton><BaseButton disabled :title="t('previewUnavailable')">{{ t('viewEdit') }}</BaseButton></div>
      <BaseAlert v-if="!ready" class="files-message">{{ t('disconnected') }}</BaseAlert>
      <BaseAlert v-if="error" class="files-message">{{ presentError(error).message }}</BaseAlert>
      <BaseAlert v-if="cleanupError" class="files-message">{{ t('cursorCleanup') }}{{ presentError(cleanupError).message }}</BaseAlert>
      <p v-if="notice" class="files-message" role="status">{{ notice }}</p>
      <div class="files-table-scroll" :aria-busy="pending"><table class="files-table"><thead><tr><th scope="col">{{ t('name') }}</th><th scope="col">{{ t('size') }}</th><th scope="col">{{ t('modified') }}</th><th scope="col">{{ t('actions') }}</th></tr></thead><tbody>
        <tr v-for="entry in entries" :key="entry.path" :class="{ selected: selectedPath === entry.path }"><td><button class="file-name" :disabled="unavailable" :aria-pressed="selectedPath === entry.path" :title="entry.path" @click="selectedPath = entry.path" @dblclick="files.open(entry)" @keydown.enter.prevent="files.open(entry)"><span aria-hidden="true">{{ entry.isSymlink ? '↗' : entry.fileType === 'directory' ? '▰' : '▤' }}</span>{{ entry.name }}</button><button v-if="entry.fileType === 'directory' || entry.isSymlink" class="file-open" :disabled="unavailable" :aria-label="t('openName', {name: entry.name})" @click="files.open(entry)">{{ t('open') }}</button></td><td :title="entry.sizeBytes ?? ''">{{ entry.fileType === 'directory' ? '—' : formatSize(entry.sizeBytes) }}</td><td>{{ modifiedTime(entry.modifiedAtMs) }}</td><td><FileMenu :name="entry.name" :disabled="unavailable" :downloadable="downloadable(entry) && !transfers.starting.value[connectionId]" @action="action($event, entry)" /></td></tr>
      </tbody></table><p v-if="pending" class="files-empty" role="status">{{ t('readingDirectory') }}</p><p v-else-if="!entries.length && !error" class="files-empty">{{ t('directoryIsEmpty') }}</p></div>
      <footer class="files-pagination"><span>{{ t('pagination', {page, count: entries.length}) }}</span><BaseButton :disabled="!ready || pending || mutating || page <= 1" @click="navigate(path)">{{ t('firstPage') }}</BaseButton><BaseButton :disabled="unavailable || !cursor" @click="files.load(path, true)">{{ t('nextPage') }}</BaseButton></footer>
    </section>
    <TransferPanel :store="transfers" :connection-id="connectionId" />
    <component :is="operation === 'delete' ? BaseAlertDialog : BaseDialog" :open="!!operation" :title="operation === 'mkdir' ? t('newFolder') : operation === 'rename' ? t('renameFile') : t('confirmDeletion')" :busy="mutating" @close="operation = null">
      <p v-if="operation === 'delete'">{{ t('deleteNote', {name: target?.name ?? '—'}) }}</p>
      <form v-else id="file-operation" @submit.prevent="submit"><label class="file-name-field">{{ t('name') }}<input v-model="name" :aria-label="t('fileName')" :disabled="mutating" /></label><p v-if="!validBasename(name)">{{ t('nameInvalid') }}</p></form>
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton data-dialog-cancel :disabled="mutating" @click="operation = null">{{ t('cancel') }}</BaseButton><BaseButton :variant="operation === 'delete' ? 'danger' : 'primary'" :disabled="unavailable || (operation !== 'delete' && !validBasename(name))" @click="submit">{{ operation === 'delete' ? t('confirmDeletion2') : t('save') }}</BaseButton></template>
    </component>
  </div>
</template>
