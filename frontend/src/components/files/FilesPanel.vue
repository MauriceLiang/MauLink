<script setup lang="ts">
import BaseAlert from "../base/BaseAlert.vue";
import BaseAlertDialog from "../base/BaseAlertDialog.vue";
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import type { RemoteFileEntry } from "../../../../contracts/v1/RemoteFileEntry";
import type { createSftpApi } from "../../ipc/sftp";
import type { TransferStore } from "../../stores/transfers";
import { createFilesStore } from "../../stores/files";
import type { TerminalPreferences } from "../../terminal/preferences";
import { defaultSettings } from "../../terminal/preferences";
import { breadcrumbs, formatSize, modifiedTime, parentRemotePath, validBasename } from "../../files/path";
import { mapError } from "../../errors/mapper";
import { presentError } from "../../errors/presenter";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import BaseIconButton from "../base/BaseIconButton.vue";
import BaseDialog from "../base/BaseDialog.vue";
import FileMenu from "./FileMenu.vue";
import RemoteTextFileDialog from "./RemoteTextFileDialog.vue";
import TransferPanel from "./TransferPanel.vue";
const t = messages(filesMessages);
const props = defineProps<{ api: ReturnType<typeof createSftpApi>; transfers: TransferStore; preferences: TerminalPreferences; connectionId: string; visible: boolean; ready: boolean; compactFooter?: boolean }>();
const emit = defineEmits<{ pagination: [value: string] }>();
const files = createFilesStore(props.api, props.connectionId);
const { entries, path, cursor, page, pending, mutating, needsReload, error, cleanupError } = files;
const location = ref(''); const selectedPath = ref<string | null>(null); const pathInput = ref<HTMLInputElement>();
const appSettings = computed(() => props.preferences.record.value?.value ?? defaultSettings);
const showSizeColumn = computed(() => appSettings.value.showSizeColumn);
const showFileSizes = computed(() => appSettings.value.showFileSizes);
const showFolderSizes = computed(() => showSizeColumn.value && appSettings.value.showFolderSizes);
const selected = computed(() => entries.value.find(entry => entry.path === selectedPath.value));
const unavailable = computed(() => !props.ready || pending.value || mutating.value || needsReload.value);
const operation = ref<'mkdir' | 'rename' | 'delete' | null>(null);
const target = shallowRef<RemoteFileEntry | null>(null); const name = ref(''); const notice = ref('');
const fileToOpen = shallowRef<RemoteFileEntry | null>(null);
const uploadRefresh = ref(false); const completedUploads = new Set<string>();
type FolderSizeState = { status: 'loading' | 'unavailable' } | { status: 'ready'; bytes: string };
const folderSizes = shallowRef<Record<string, FolderSizeState>>({});
let folderSizeGeneration = 0; let disposed = false;
watch(path, value => { location.value = value; });
watch(entries, () => { selectedPath.value = null; });
watch([page, entries], () => emit('pagination', t('pagination', {page: page.value, count: entries.value.length})), { immediate: true });
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
function openEntry(entry: RemoteFileEntry) {
  if (unavailable.value) return;
  if (entry.fileType === 'directory' || entry.isSymlink || entry.fileType === 'symlink') { void files.open(entry); return; }
  if (entry.fileType === 'file') fileToOpen.value = { ...entry };
}
function openEntryFromRow(entry: RemoteFileEntry, event: MouseEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest('button') && !target.closest('.file-name, .file-open')) return;
  openEntry(entry);
}
function viewable(entry: RemoteFileEntry | undefined) { return entry?.fileType === 'file' && !entry.isSymlink; }
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
async function action(value: 'download' | 'rename' | 'delete' | 'copy' | 'view', entry: RemoteFileEntry) {
  if (unavailable.value) return;
  if (value === 'download') await transfer('download', entry);
  else if (value === 'view') openEntry(entry);
  else if (value === 'copy') {
    try { await navigator.clipboard.writeText(entry.path); notice.value = t('pathCopied'); }
    catch (reason) { error.value = mapError(reason); }
  } else edit(value, entry);
}
const downloadable = (entry: RemoteFileEntry) => entry.fileType === 'file' || entry.isSymlink || entry.fileType === 'symlink';
function setFolderSize(path: string, state: FolderSizeState, generation: number) {
  if (generation === folderSizeGeneration && !disposed) folderSizes.value = { ...folderSizes.value, [path]: state };
}
async function calculateFolderSize(path: string, generation: number, cache: Map<string, Promise<bigint | null>>): Promise<bigint | null> {
  const cached = cache.get(path); if (cached) return cached;
  const result = (async () => {
    if (generation !== folderSizeGeneration || disposed) return null;
    setFolderSize(path, { status: 'loading' }, generation);
    const subdirectories: string[] = []; let total = BigInt(0); let complete = true; let cursorId: string | null = null;
    const collect = (values: RemoteFileEntry[]) => {
      for (const entry of values) {
        if (entry.fileType === 'file') {
          if (entry.sizeBytes === null) complete = false;
          else total += BigInt(entry.sizeBytes);
        } else if (entry.fileType === 'directory' && !entry.isSymlink) subdirectories.push(entry.path);
      }
    };
    try {
      let result = await props.api.listStart({ connectionId: props.connectionId, path });
      cursorId = result.cursorId; collect(result.entries);
      while (cursorId && generation === folderSizeGeneration && !disposed) {
        result = await props.api.listNext({ cursorId });
        cursorId = result.cursorId; collect(result.entries);
      }
      if (generation !== folderSizeGeneration || disposed) return null;
      if (!complete) { setFolderSize(path, { status: 'unavailable' }, generation); return null; }
    } catch {
      setFolderSize(path, { status: 'unavailable' }, generation); return null;
    } finally {
      if (cursorId) {
        try { await props.api.listClose({ cursorId }); }
        catch (reason) { if (generation === folderSizeGeneration && !disposed && !cleanupError.value) cleanupError.value = mapError(reason); }
      }
    }
    for (const childPath of subdirectories) {
      const childSize = await calculateFolderSize(childPath, generation, cache);
      if (childSize === null) { setFolderSize(path, { status: 'unavailable' }, generation); return null; }
      total += childSize;
    }
    if (generation !== folderSizeGeneration || disposed) return null;
    setFolderSize(path, { status: 'ready', bytes: total.toString() }, generation);
    return total;
  })();
  cache.set(path, result); return result;
}
function displaySize(entry: RemoteFileEntry) {
  if (entry.fileType === 'file') return showFileSizes.value ? formatSize(entry.sizeBytes) : '';
  if (entry.fileType !== 'directory' || !showFolderSizes.value) return '';
  const state = folderSizes.value[entry.path];
  if (state?.status === 'ready') return formatSize(state.bytes);
  if (state?.status === 'unavailable') return '—';
  return t('calculatingDirectorySize');
}
watch([entries, showFolderSizes, () => props.visible, () => props.ready, pending], ([currentEntries, enabled, visible, ready, loading]) => {
  const generation = ++folderSizeGeneration; folderSizes.value = {};
  const directories = currentEntries.filter(entry => entry.fileType === 'directory' && !entry.isSymlink);
  if (!enabled || !visible || !ready || loading || !directories.length) return;
  folderSizes.value = Object.fromEntries(directories.map(entry => [entry.path, { status: 'loading' as const }]));
  const cache = new Map<string, Promise<bigint | null>>();
  void (async () => { for (const entry of directories) { if (generation !== folderSizeGeneration) return; await calculateFolderSize(entry.path, generation, cache); } })();
}, { immediate: true });
onBeforeUnmount(() => { disposed = true; ++folderSizeGeneration; files.dispose(); });
</script>
<template>
  <div class="files-workspace">
    <section class="files-browser" :aria-label="t('remoteFiles')">
      <form class="files-location" @submit.prevent="navigate(location)"><BaseIconButton class="files-parent-action" :label="t('parentDirectory')" :disabled="!ready || pending || mutating || !path || path === '/'" @click="navigate(parentRemotePath(path))"><BaseIcon name="arrow-up" /></BaseIconButton><input ref="pathInput" v-model="location" :aria-label="t('remotePath')" :disabled="!ready || mutating" :placeholder="t('remotePathEGHome')" @keydown.enter.prevent="navigate(location)" /><BaseIconButton class="files-go" :label="t('go')" type="submit" :disabled="!ready || pending || mutating"><BaseIcon name="arrow-right" /></BaseIconButton><div class="files-toolbar" role="group" :aria-label="t('actions')"><BaseIconButton class="files-toolbar-action" :label="t('uploadFile')" :disabled="unavailable || transfers.starting.value[connectionId]" @click="transfer('upload')"><BaseIcon name="upload" /></BaseIconButton><BaseIconButton class="files-toolbar-action" :label="t('newFolder')" :disabled="unavailable" @click="edit('mkdir')"><BaseIcon name="folder-plus" /></BaseIconButton><BaseIconButton class="files-toolbar-action" :label="t('downloadFile')" :disabled="unavailable || !selected || !downloadable(selected) || transfers.starting.value[connectionId]" @click="selected && transfer('download', selected)"><BaseIcon name="download" /></BaseIconButton><BaseIconButton class="files-toolbar-action" :label="t('viewEdit')" :disabled="unavailable || !viewable(selected)" @click="selected && openEntry(selected)"><BaseIcon name="eye" /></BaseIconButton></div><BaseIconButton class="files-refresh" :label="t('refreshDirectory')" :disabled="!ready || pending || mutating" @click="navigate(path || '.')"><BaseIcon name="refresh" /></BaseIconButton></form>
      <nav class="files-breadcrumb" :aria-label="t('directoryNavigation')"><button v-for="part in breadcrumbs(path)" :key="part.path" :aria-current="part.path === path ? 'location' : undefined" :disabled="!ready || pending || mutating" @click="navigate(part.path)">{{ part.name }}</button></nav>
      <BaseAlert v-if="!ready" class="files-message">{{ t('disconnected') }}</BaseAlert>
      <BaseAlert v-if="error" class="files-message">{{ presentError(error).message }}</BaseAlert>
      <BaseAlert v-if="cleanupError" class="files-message">{{ t('cursorCleanup') }}{{ presentError(cleanupError).message }}</BaseAlert>
      <p v-if="notice" class="files-message" role="status">{{ notice }}</p>
      <div class="files-table-scroll" :aria-busy="pending"><table class="files-table"><thead><tr><th scope="col">{{ t('name') }}</th><th v-if="showSizeColumn" scope="col">{{ t('size') }}</th><th scope="col">{{ t('modified') }}</th><th scope="col">{{ t('actions') }}</th></tr></thead><tbody>
        <tr v-for="entry in entries" :key="entry.path" :class="{ selected: selectedPath === entry.path }" @click="selectedPath = entry.path" @dblclick="openEntryFromRow(entry, $event)"><td><button class="file-name" :disabled="unavailable" :aria-pressed="selectedPath === entry.path" :title="entry.path" @click="selectedPath = entry.path" @keydown.enter.prevent="openEntry(entry)"><span class="file-entry-icon" aria-hidden="true"><BaseIcon v-if="entry.fileType === 'directory' && !entry.isSymlink" name="folder" /><BaseIcon v-else-if="entry.fileType === 'file' && !entry.isSymlink" name="file-text" /><span v-else>↗</span></span>{{ entry.name }}</button><button v-if="entry.fileType === 'directory' || entry.isSymlink" class="file-open" :disabled="unavailable" :aria-label="t('openName', {name: entry.name})" @click="files.open(entry)">{{ t('open') }}</button></td><td v-if="showSizeColumn">{{ displaySize(entry) }}</td><td>{{ modifiedTime(entry.modifiedAtMs) }}</td><td><FileMenu :name="entry.name" :disabled="unavailable" :downloadable="downloadable(entry) && !transfers.starting.value[connectionId]" :viewable="viewable(entry)" @action="action($event, entry)" /></td></tr>
      </tbody></table><p v-if="pending" class="files-empty" role="status">{{ t('readingDirectory') }}</p><p v-else-if="!entries.length && !error" class="files-empty">{{ t('directoryIsEmpty') }}</p></div>
      <footer class="files-pagination"><span v-if="!compactFooter">{{ t('pagination', {page, count: entries.length}) }}</span><BaseButton :disabled="!ready || pending || mutating || page <= 1" @click="navigate(path)">{{ t('firstPage') }}</BaseButton><BaseButton :disabled="unavailable || !cursor" @click="files.load(path, true)">{{ t('nextPage') }}</BaseButton></footer>
    </section>
    <TransferPanel :store="transfers" :connection-id="connectionId" />
    <component :is="operation === 'delete' ? BaseAlertDialog : BaseDialog" :open="!!operation" :title="operation === 'mkdir' ? t('newFolder') : operation === 'rename' ? t('renameFile') : t('confirmDeletion')" :busy="mutating" @close="operation = null">
      <p v-if="operation === 'delete'">{{ t('deleteNote', {name: target?.name ?? '—'}) }}</p>
      <form v-else id="file-operation" @submit.prevent="submit"><label class="file-name-field">{{ t('name') }}<input v-model="name" :aria-label="t('fileName')" :disabled="mutating" /></label><p v-if="!validBasename(name)">{{ t('nameInvalid') }}</p></form>
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton data-dialog-cancel :disabled="mutating" @click="operation = null">{{ t('cancel') }}</BaseButton><BaseButton :variant="operation === 'delete' ? 'danger' : 'primary'" :disabled="unavailable || (operation !== 'delete' && !validBasename(name))" @click="submit">{{ operation === 'delete' ? t('confirmDeletion2') : t('save') }}</BaseButton></template>
    </component>
    <RemoteTextFileDialog v-if="fileToOpen" :api="api" :connection-id="connectionId" :entry="fileToOpen" :open="true" @close="fileToOpen = null" @saved="files.load()" @open-linked-file="fileToOpen = $event" />
  </div>
</template>
