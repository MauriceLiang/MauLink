<script setup lang="ts">
import { onMounted, ref } from "vue";
import BaseButton from "../components/base/BaseButton.vue";
import BaseIconButton from "../components/base/BaseIconButton.vue";
import BaseInput from "../components/base/BaseInput.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import BaseEmptyState from "../components/base/BaseEmptyState.vue";
import BaseStatusBadge from "../components/base/BaseStatusBadge.vue";
import { createIpcClient } from "../ipc/client";
import { createServerApi } from "../ipc/server";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { createFoundationMock } from "./fixtures";

const server = createServerApi(createIpcClient(createFoundationMock()));
const pending = ref(false);
const count = ref<number | null>(null);
const error = ref<ReturnType<typeof presentError> | null>(null);
const open = ref(false);
const busy = ref(false);
const actionDisabled = ref(false);
const actions = ref(0);
const name = ref("");
const lastFocus = ref("");
async function load(fail = false) {
  pending.value = true;
  error.value = null;
  try {
    const page = await server.list({ query: fail ? "error" : null, groupId: null, limit: 20, cursor: null });
    count.value = page.items.length;
  } catch (failure) {
    error.value = presentError(mapError(failure));
  } finally {
    pending.value = false;
  }
}
onMounted(() => load());
</script>

<template>
  <main class="foundations-harness">
    <h1>基础组件与 Mock IPC 验收</h1>
    <p role="status">{{ pending ? '正在加载 Mock IPC…' : `浏览器 Mock IPC 已加载：${count ?? '—'} 台服务器` }}</p>
    <p>动作触发次数：{{ actions }}。Tab 应跳过禁用/加载按钮；对话框末尾焦点是输入框。</p>
    <label><input v-model="actionDisabled" type="checkbox" /> 动态禁用动作按钮</label>
    <div class="harness-actions">
      <BaseButton :disabled="actionDisabled" @click="actions++">键盘动作按钮</BaseButton>
      <BaseButton variant="primary" @click="open = true">打开基础对话框</BaseButton>
      <BaseButton disabled @click="actions++">禁用按钮</BaseButton>
      <BaseButton loading @click="actions++">加载中</BaseButton>
      <BaseIconButton label="刷新 Mock IPC" :disabled="pending" @click="load()">↻</BaseIconButton>
      <BaseButton :disabled="pending" @click="load(true)">模拟 IPC 错误</BaseButton>
      <BaseStatusBadge tone="success">仅使用 Mock IPC</BaseStatusBadge>
    </div>
    <BaseInput v-model="name" label="示例名称" placeholder="支持键盘输入" />
    <BaseEmptyState title="暂无服务器" description="这是开发验收 fixture，不会访问真实服务器。" />
    <BaseAlert v-if="error">{{ error.message }}</BaseAlert>
    <BaseButton v-if="error?.retryable" @click="load()">重试</BaseButton>
    <BaseDialog :open="open" :busy="busy" title="基础对话框" @close="open = false">
      <BaseInput v-model="name" label="示例名称" />
      <label><input v-model="busy" type="checkbox" /> busy（Esc 不关闭；取消勾选恢复普通状态）</label>
      <template #footer>
        <BaseButton :disabled="busy" @click="open = false">关闭</BaseButton>
        <BaseInput v-model="lastFocus" label="末尾焦点输入" />
        <BaseButton disabled tabindex="0" @click="actions++">对话框禁用按钮</BaseButton>
      </template>
    </BaseDialog>
  </main>
</template>

<style scoped>
.foundations-harness { max-width: 900px; margin: 0 auto; padding: 64px 24px 24px; }
.harness-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; margin: 20px 0; }
</style>
