<script setup lang="ts">
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
  if (success) { operation.value = null; notice.value = '文件操作已完成'; await files.load(); await nextTick(); pathInput.value?.focus(); }
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
    try { await navigator.clipboard.writeText(entry.path); notice.value = '路径已复制'; }
    catch (reason) { error.value = mapError(reason); }
  } else edit(value, entry);
}
const downloadable = (entry: RemoteFileEntry) => entry.fileType === 'file' || entry.isSymlink || entry.fileType === 'symlink';
onBeforeUnmount(files.dispose);
</script>
<template>
  <div class="files-workspace">
    <section class="files-browser" aria-label="远程文件">
      <div class="files-heading"><h2>远程文件</h2><span>SFTP</span><BaseButton :disabled="!ready || pending || mutating" @click="navigate(path || '.')">刷新目录</BaseButton></div>
      <form class="files-location" @submit.prevent="navigate(location)"><BaseButton :disabled="!ready || pending || mutating || !path || path === '/'" aria-label="上级目录" @click="navigate(parentRemotePath(path))">↑</BaseButton><input ref="pathInput" v-model="location" aria-label="远程路径" :disabled="!ready || mutating" placeholder="远程路径，例如 /home" /><BaseButton type="submit" :disabled="!ready || pending || mutating">前往</BaseButton></form>
      <nav class="files-breadcrumb" aria-label="目录导航"><button v-for="part in breadcrumbs(path)" :key="part.path" :aria-current="part.path === path ? 'location' : undefined" :disabled="!ready || pending || mutating" @click="navigate(part.path)">{{ part.name }}</button></nav>
      <div class="files-toolbar"><BaseButton :disabled="unavailable || transfers.starting.value[connectionId]" @click="transfer('upload')">上传文件</BaseButton><BaseButton :disabled="unavailable" @click="edit('mkdir')">新建文件夹</BaseButton><BaseButton :disabled="unavailable || !selected || !downloadable(selected) || transfers.starting.value[connectionId]" @click="selected && transfer('download', selected)">下载文件</BaseButton><BaseButton disabled title="当前 Core 尚未提供远程文件预览或编辑接口">查看 / 编辑</BaseButton></div>
      <p v-if="!ready" class="files-message" role="alert">SSH 连接已结束，文件操作不可用。</p>
      <p v-if="error" class="files-message" role="alert">{{ presentError(error).message }}</p>
      <p v-if="cleanupError" class="files-message" role="alert">目录游标释放失败：{{ presentError(cleanupError).message }}</p>
      <p v-if="notice" class="files-message" role="status">{{ notice }}</p>
      <div class="files-table-scroll" :aria-busy="pending"><table class="files-table"><thead><tr><th scope="col">名称</th><th scope="col">大小</th><th scope="col">修改时间</th><th scope="col">操作</th></tr></thead><tbody>
        <tr v-for="entry in entries" :key="entry.path" :class="{ selected: selectedPath === entry.path }"><td><button class="file-name" :disabled="unavailable" :aria-pressed="selectedPath === entry.path" :title="entry.path" @click="selectedPath = entry.path" @dblclick="files.open(entry)" @keydown.enter.prevent="files.open(entry)"><span aria-hidden="true">{{ entry.isSymlink ? '↗' : entry.fileType === 'directory' ? '▰' : '▤' }}</span>{{ entry.name }}</button><button v-if="entry.fileType === 'directory' || entry.isSymlink" class="file-open" :disabled="unavailable" :aria-label="`打开 ${entry.name}`" @click="files.open(entry)">打开</button></td><td :title="entry.sizeBytes ?? ''">{{ entry.fileType === 'directory' ? '—' : formatSize(entry.sizeBytes) }}</td><td>{{ modifiedTime(entry.modifiedAtMs) }}</td><td><FileMenu :name="entry.name" :disabled="unavailable" :downloadable="downloadable(entry) && !transfers.starting.value[connectionId]" @action="action($event, entry)" /></td></tr>
      </tbody></table><p v-if="pending" class="files-empty" role="status">正在读取目录…</p><p v-else-if="!entries.length && !error" class="files-empty">目录为空</p></div>
      <footer class="files-pagination"><span>第 {{ page }} 页 · 本页 {{ entries.length }} 项</span><BaseButton :disabled="!ready || pending || mutating || page <= 1" @click="navigate(path)">回到第一页</BaseButton><BaseButton :disabled="unavailable || !cursor" @click="files.load(path, true)">下一页</BaseButton></footer>
    </section>
    <TransferPanel :store="transfers" :connection-id="connectionId" />
    <BaseDialog :open="!!operation" :title="operation === 'mkdir' ? '新建文件夹' : operation === 'rename' ? '重命名文件' : '确认删除？'" :busy="mutating" @close="operation = null">
      <p v-if="operation === 'delete'">确认删除“{{ target?.name }}”？此操作无法撤销。目录必须为空；符号链接只删除链接。</p>
      <form v-else id="file-operation" @submit.prevent="submit"><label class="file-name-field">名称<input v-model="name" aria-label="文件名称" :disabled="mutating" /></label><p v-if="!validBasename(name)">名称不能为空，也不能是 .、.. 或包含 /。</p></form>
      <p v-if="error" role="alert">{{ presentError(error).message }}</p>
      <template #footer><BaseButton :disabled="mutating" @click="operation = null">取消</BaseButton><BaseButton :variant="operation === 'delete' ? 'danger' : 'primary'" :disabled="unavailable || (operation !== 'delete' && !validBasename(name))" @click="submit">{{ operation === 'delete' ? '确认删除' : '保存' }}</BaseButton></template>
    </BaseDialog>
  </div>
</template>
