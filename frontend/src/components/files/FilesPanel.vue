<script setup lang="ts">
import BaseAlert from "../base/BaseAlert.vue";
import BaseAlertDialog from "../base/BaseAlertDialog.vue";
import { messages } from "../../i18n/locale";
import { filesMessages } from "../../i18n/files";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
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
import BaseDropdownMenu from "../base/BaseDropdownMenu.vue";
import type { MenuItem } from "../base/menu";
import BaseDialog from "../base/BaseDialog.vue";
import FileMenu from "./FileMenu.vue";
import RemoteTextFileDialog from "./RemoteTextFileDialog.vue";
const t = messages(filesMessages);
const props = defineProps<{ api: ReturnType<typeof createSftpApi>; transfers: TransferStore; preferences: TerminalPreferences; connectionId: string; visible: boolean; ready: boolean; inline?: boolean; requestedDirectory?: { path: string; revision: number } | null }>();
const emit = defineEmits<{ collapse: [] }>();
const files = createFilesStore(props.api, props.connectionId);
const { entries, path, cursor, page, pending, mutating, needsReload, error, cleanupError } = files;
const filesBrowser = ref<HTMLElement | null>(null); const pathDisplay = ref<HTMLButtonElement>(); const pathInput = ref<HTMLInputElement>();
const location = ref(''); const pathEditing = ref(false); const narrowToolbar = ref(false); const selectedPath = ref<string | null>(null);
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
let directoryRevision: number | undefined;
watch(path, value => { if (!pathEditing.value) location.value = value; });
watch(entries, () => { selectedPath.value = null; });
watch(() => props.ready, ready => { if (!ready) files.suspend(); });
watch([() => props.visible, () => props.ready, () => props.requestedDirectory, mutating], ([visible, ready]) => {
  if (!visible || !ready || mutating.value) return;
  const directory = props.requestedDirectory;
  if (directory && directory.revision !== directoryRevision) {
    directoryRevision = directory.revision;
    notice.value = '';
    void files.load(directory.path);
  } else if (!pending.value && (!path.value || needsReload.value)) void files.load(path.value || '.');
}, { immediate: true });
watch(() => props.visible, visible => {
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
async function navigate(destination: string) {
  if (!props.ready || mutating.value) return;
  notice.value = '';
  await files.load(destination);
  pathEditing.value = false;
  location.value = path.value;
  await nextTick();
  pathDisplay.value?.focus();
}
async function beginPathEdit() { location.value = path.value; pathEditing.value = true; await nextTick(); pathInput.value?.focus(); pathInput.value?.select(); }
async function cancelPathEdit() { pathEditing.value = false; location.value = path.value; await nextTick(); pathDisplay.value?.focus(); }
function openEntry(entry: RemoteFileEntry) {
  if (unavailable.value) return;
  if (entry.fileType === 'directory' || entry.isSymlink || entry.fileType === 'symlink') { void files.open(entry); return; }
  if (entry.fileType === 'file') fileToOpen.value = { ...entry };
}
function selectContextRow(entry: RemoteFileEntry, event: MouseEvent) {
  selectedPath.value = entry.path;
  if (unavailable.value) event.preventDefault();
}
function openContextMenuFromKeyboard(event: KeyboardEvent) {
  if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10')) return;
  event.preventDefault();
  const row = event.currentTarget as HTMLTableRowElement;
  const rect = row.getBoundingClientRect();
  row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: rect.left + 16, clientY: rect.top + rect.height / 2 }));
}
function openEntryFromRow(entry: RemoteFileEntry, event: MouseEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest('button') && !target.closest('.file-name')) return;
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
  if (success) { operation.value = null; notice.value = t('fileOperationCompleted'); await nextTick(); if (props.ready && !pending.value) await files.load(); await nextTick(); pathDisplay.value?.focus(); }
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
const ancestorItems = computed<MenuItem[]>(() => breadcrumbs(path.value).map(part => ({ id: part.path, label: part.name, disabled: unavailable.value || part.path === path.value, title: part.path })));
const toolbarMenuItems = computed<MenuItem[]>(() => [
  ...(narrowToolbar.value ? [{ id: 'new-folder', label: t('newFolder'), disabled: unavailable.value }] : []),
  { id: 'download', label: t('downloadFile'), disabled: unavailable.value || !selected.value || !downloadable(selected.value) || !!props.transfers.starting.value[props.connectionId] },
  { id: 'view', label: t('viewEdit'), disabled: unavailable.value || !viewable(selected.value) },
]);
async function toolbarAction(id: string) {
  if (id === 'new-folder') edit('mkdir');
  else if (id === 'download' && selected.value) await transfer('download', selected.value);
  else if (id === 'view' && selected.value) openEntry(selected.value);
}
let toolbarResizeObserver: ResizeObserver | undefined;
onMounted(() => {
  const updateWidth = () => { const width = filesBrowser.value?.getBoundingClientRect().width ?? 0; if (width > 0) narrowToolbar.value = width <= 540; };
  if (filesBrowser.value && typeof ResizeObserver !== 'undefined') {
    toolbarResizeObserver = new ResizeObserver(updateWidth);
    toolbarResizeObserver.observe(filesBrowser.value);
  }
  updateWidth();
});
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
onBeforeUnmount(() => { disposed = true; ++folderSizeGeneration; toolbarResizeObserver?.disconnect(); files.dispose(); });
</script>
<template>
  <div class="files-workspace">
    <section ref="filesBrowser" class="files-browser" :aria-label="t('remoteFiles')">
      <form class="files-location" @submit.prevent="navigate(location)">
        <BaseIconButton class="files-parent-action" :label="t('parentDirectory')" :disabled="!ready || pending || mutating || !path || path === '/'" @click="navigate(parentRemotePath(path))"><BaseIcon name="arrow-up" /></BaseIconButton>
        <input v-if="pathEditing" ref="pathInput" v-model="location" :aria-label="t('remotePath')" :disabled="!ready || mutating" :placeholder="t('remotePathEGHome')" @keydown.esc.prevent="cancelPathEdit" />
        <button v-else ref="pathDisplay" type="button" class="files-path-display" :title="path" :aria-label="t('editRemotePath', {path})" :disabled="!ready || mutating" @click="beginPathEdit">{{ path || '.' }}</button>
        <div class="files-ancestors"><BaseDropdownMenu :label="t('directoryNavigation')" :disabled="!ready || pending || mutating" :items="ancestorItems" @action="navigate" /></div>
        <BaseIconButton v-if="pathEditing" class="files-go" :label="t('go')" type="submit" :disabled="!ready || pending || mutating"><BaseIcon name="arrow-right" /></BaseIconButton>
        <div class="files-toolbar" role="group" :aria-label="t('actions')">
          <BaseIconButton class="files-toolbar-action" :label="t('uploadFile')" :disabled="unavailable || transfers.starting.value[connectionId]" @click="transfer('upload')"><BaseIcon name="upload" /></BaseIconButton>
          <BaseIconButton v-if="!narrowToolbar" class="files-toolbar-action files-new-folder" :label="t('newFolder')" :disabled="unavailable" @click="edit('mkdir')"><BaseIcon name="folder-plus" /></BaseIconButton>
          <BaseDropdownMenu :label="t('moreActions')" :disabled="unavailable" :items="toolbarMenuItems" @action="toolbarAction" />
          <BaseIconButton class="files-toolbar-action" :label="t('refreshDirectory')" :disabled="!ready || pending || mutating" @click="navigate(path || '.')"><BaseIcon name="refresh" /></BaseIconButton>
          <BaseIconButton v-if="inline" class="files-toolbar-action files-collapse-action" :label="t('collapseInlineFiles')" @click="emit('collapse')"><BaseIcon name="minimize" /></BaseIconButton>
        </div>
      </form>
      <BaseAlert v-if="!ready" class="files-message">{{ t('disconnected') }}</BaseAlert>
      <BaseAlert v-if="error" class="files-message">{{ presentError(error).message }}</BaseAlert>
      <BaseAlert v-if="cleanupError" class="files-message">{{ t('cursorCleanup') }}{{ presentError(cleanupError).message }}</BaseAlert>
      <p v-if="notice" class="files-message" role="status">{{ notice }}</p>
      <div class="files-table-scroll" :aria-busy="pending"><table class="files-table" :class="{ 'has-size-column': showSizeColumn }"><thead><tr><th scope="col">{{ t('name') }}</th><th v-if="showSizeColumn" scope="col">{{ t('size') }}</th><th scope="col">{{ t('modified') }}</th></tr></thead><tbody>
        <FileMenu v-for="entry in entries" :key="entry.path" :name="entry.name" context-menu :disabled="unavailable" :downloadable="downloadable(entry) && !transfers.starting.value[connectionId]" :viewable="viewable(entry)" @action="action($event, entry)">
          <tr :class="{ selected: selectedPath === entry.path }" @click="selectedPath = entry.path" @contextmenu="selectContextRow(entry, $event)" @keydown="openContextMenuFromKeyboard($event)" @dblclick="openEntryFromRow(entry, $event)"><td><button class="file-name" :disabled="unavailable" :aria-pressed="selectedPath === entry.path" :title="entry.path" @click="selectedPath = entry.path" @keydown.enter.prevent="openEntry(entry)"><span class="file-entry-icon" aria-hidden="true"><BaseIcon v-if="entry.fileType === 'directory' && !entry.isSymlink" name="folder" /><BaseIcon v-else-if="entry.fileType === 'file' && !entry.isSymlink" name="file-text" /><span v-else>↗</span></span><span class="file-name-label">{{ entry.name }}</span></button><button v-if="entry.fileType === 'directory' || entry.isSymlink" class="file-open" :disabled="unavailable" :aria-label="t('openName', {name: entry.name})" @click="files.open(entry)">{{ t('open') }}</button></td><td v-if="showSizeColumn" :title="displaySize(entry)">{{ displaySize(entry) }}</td><td :title="entry.modifiedAtMs === null ? '' : new Date(entry.modifiedAtMs).toLocaleString()">{{ modifiedTime(entry.modifiedAtMs) }}</td></tr>
        </FileMenu>
      </tbody></table><p v-if="pending" class="files-empty" role="status">{{ t('readingDirectory') }}</p><p v-else-if="!entries.length && !error" class="files-empty">{{ t('directoryIsEmpty') }}</p></div>
      <footer v-if="page > 1 || cursor" class="files-pagination"><span>{{ t('pagination', {page, count: entries.length}) }}</span><BaseButton v-if="page > 1" :disabled="!ready || pending || mutating" @click="navigate(path)">{{ t('firstPage') }}</BaseButton><BaseButton v-if="cursor" :disabled="unavailable" @click="files.load(path, true)">{{ t('nextPage') }}</BaseButton></footer>
    </section>
    <component :is="operation === 'delete' ? BaseAlertDialog : BaseDialog" :open="!!operation" :size="operation === 'delete' ? 'compact' : 'standard'" :variant="operation === 'delete' ? 'confirmation' : 'form'" :title="operation === 'mkdir' ? t('newFolder') : operation === 'rename' ? t('renameFile') : t('confirmDeletion')" :busy="mutating" @close="operation = null">
      <template v-if="operation === 'delete'"><p>{{ t('deleteNote', {name: target?.name ?? '—'}) }}</p><code v-if="target?.path" class="file-delete-path">{{ target.path }}</code></template>
      <form v-else id="file-operation" @submit.prevent="submit"><label class="file-name-field">{{ t('name') }}<input v-model="name" :aria-label="t('fileName')" :disabled="mutating" /></label><p v-if="!validBasename(name)">{{ t('nameInvalid') }}</p></form>
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton data-dialog-cancel :disabled="mutating" @click="operation = null">{{ t('cancel') }}</BaseButton><BaseButton :variant="operation === 'delete' ? 'danger' : 'neutralPrimary'" :disabled="unavailable || (operation !== 'delete' && !validBasename(name))" @click="submit">{{ operation === 'delete' ? t('confirmDeletion2') : t('save') }}</BaseButton></template>
    </component>
    <RemoteTextFileDialog v-if="fileToOpen" :api="api" :connection-id="connectionId" :entry="fileToOpen" :open="true" @close="fileToOpen = null" @saved="files.load()" @open-linked-file="fileToOpen = $event" />
  </div>
</template>
