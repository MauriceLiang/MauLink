<script setup lang="ts">
import { ref, watch } from "vue";
import type { TerminalPreferences } from "../terminal/preferences";
import { defaultSettings } from "../terminal/preferences";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseButton from "../components/base/BaseButton.vue";
const props = defineProps<{ open: boolean; preferences: TerminalPreferences }>();
const emit = defineEmits<{ close: [] }>();
const font = ref('monospace'); const size = ref(14); const cursor = ref(defaultSettings.terminalCursorStyle); const scrollback = ref(10000); const copy = ref(false);
function reset() { const value = props.preferences.record.value?.value ?? defaultSettings; font.value = value.terminalFontFamily; size.value = value.terminalFontSize; cursor.value = value.terminalCursorStyle; scrollback.value = value.terminalScrollbackLines; copy.value = props.preferences.copyOnSelect.value; }
watch(() => props.open, async open => { if (open) { await props.preferences.load(); reset(); } });
async function save() { if (await props.preferences.save({ terminalFontFamily: font.value, terminalFontSize: size.value, terminalCursorStyle: cursor.value, terminalScrollbackLines: scrollback.value }, copy.value)) emit('close'); }
</script>
<template>
  <BaseDialog :open="open" title="终端设置" :busy="preferences.busy.value" @close="emit('close')">
    <form id="terminal-settings-form" class="terminal-settings-form" @submit.prevent="save">
      <fieldset :disabled="preferences.busy.value || !preferences.record.value">
        <label>字号<input v-model.number="size" type="number" min="8" max="72" step="1" required /></label>
        <label>光标<select v-model="cursor"><option value="block">方块</option><option value="bar">竖线</option><option value="underline">下划线</option></select></label>
        <label><input v-model="copy" type="checkbox" />选中即复制</label><p class="connection-muted">默认关闭。开启后选择终端文本会写入系统剪贴板。</p>
        <label>字体<input v-model="font" maxlength="128" required /></label><label>滚动缓冲行数<input v-model.number="scrollback" type="number" min="1000" max="100000" step="1" required /></label>
      </fieldset>
    </form>
    <p v-if="preferences.error.value" class="server-form-error" role="alert">{{ preferences.error.value }}</p>
    <BaseButton v-if="preferences.error.value" :disabled="preferences.busy.value" @click="preferences.load().then(reset)">重新加载设置（放弃修改）</BaseButton>
    <template #footer><BaseButton :disabled="preferences.busy.value" @click="emit('close')">取消</BaseButton><BaseButton variant="primary" type="submit" form="terminal-settings-form" :disabled="!preferences.record.value" :loading="preferences.busy.value">保存设置</BaseButton></template>
  </BaseDialog>
</template>
