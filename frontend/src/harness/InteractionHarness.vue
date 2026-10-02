<script setup lang="ts">
import { ref, watch } from 'vue';
import BaseButton from '../components/base/BaseButton.vue';
import BaseIconButton from '../components/base/BaseIconButton.vue';
import BaseIcon from '../components/base/BaseIcon.vue';
import BaseInput from '../components/base/BaseInput.vue';
import BaseSelect from '../components/base/BaseSelect.vue';
import BaseCheckbox from '../components/base/BaseCheckbox.vue';
import BaseSwitch from '../components/base/BaseSwitch.vue';
import BaseDropdownMenu from '../components/base/BaseDropdownMenu.vue';
import BaseContextMenu from '../components/base/BaseContextMenu.vue';
import BasePopover from '../components/base/BasePopover.vue';
import BaseCollapsible from '../components/base/BaseCollapsible.vue';
import BaseDialog from '../components/base/BaseDialog.vue';
import BaseAlertDialog from '../components/base/BaseAlertDialog.vue';
import BaseAlert from '../components/base/BaseAlert.vue';
import BaseToastViewport from '../components/base/BaseToastViewport.vue';
import { useToast } from '../composables/useToast';
const theme=ref('light'); const selection=ref<string>(); const text=ref(''); const checked=ref(false); const enabled=ref(false); const advanced=ref(false); const dialog=ref(false); const alert=ref(false); const busy=ref(false); const action=ref('未执行'); const count=ref(0);
const toast=useToast();
const menu=[{id:'disabled',label:'不可用项',disabled:true},{id:'edit',label:'编辑'},{id:'delete',label:'删除',danger:true}];
watch(theme,value=>{document.documentElement.dataset.theme=value;},{immediate:true});
</script>
<template>
  <main class="interaction-harness"><h1>Reka UI · 交互验收</h1><p>DEV 组件预览，无真实 IPC、服务器或文件操作。Tab / Shift+Tab、方向键、Enter、Space、Esc 均可验收。</p>
    <BaseSelect v-model="theme" label="主题" :options="[{value:'light',label:'Light'},{value:'dark',label:'Dark'},{value:'system',label:'System'}]" />
    <section><h2>按钮与字段</h2><div class="interaction-row"><BaseButton @click="count++">普通按钮</BaseButton><BaseButton variant="primary">主要按钮</BaseButton><BaseButton variant="danger">危险按钮</BaseButton><BaseButton disabled @click="count++">禁用按钮</BaseButton><BaseButton loading>加载按钮</BaseButton><BaseIconButton label="说明"><BaseIcon name="info" /></BaseIconButton><output>动作次数 {{ count }}</output></div><BaseInput v-model="text" label="输入" /><BaseInput v-model="text" label="错误输入" error="示例错误" /><BaseInput v-model="text" label="禁用输入" disabled /></section>
    <section><h2>选择与切换</h2><BaseSelect v-model="selection" label="选择项目" placeholder="请选择" :options="[{value:'one',label:'第一项'},{value:'off',label:'禁用项',disabled:true},{value:'two',label:'第二项'}]" /><BaseSelect label="禁用选择" disabled :options="[{value:'one',label:'第一项'}]" /><BaseCheckbox v-model="checked" label="确认选项" /><BaseCheckbox :model-value="true" label="禁用勾选" disabled /><BaseSwitch v-model="enabled" label="启用选项" /><BaseCollapsible v-model:open="advanced" label="高级设置"><BaseInput v-model="text" label="高级输入" /></BaseCollapsible></section>
    <section><h2>菜单与浮层</h2><div class="interaction-row"><BaseDropdownMenu label="操作菜单" :items="menu" @action="action=$event" /><BaseContextMenu label="右键菜单" :items="menu" @action="action=$event"><button class="interaction-context">在此右键</button></BaseContextMenu><BasePopover title="说明"><template #trigger><BaseButton>打开 Popover</BaseButton></template><p>不阻塞页面的轻量说明。</p><BaseInput v-model="text" label="局部输入" /></BasePopover><output>菜单动作 {{ action }}</output></div></section>
    <section><h2>对话框与反馈</h2><div class="interaction-row"><BaseButton @click="dialog=true">打开 Dialog</BaseButton><BaseButton variant="danger" @click="alert=true">打开 AlertDialog</BaseButton><BaseButton @click="toast.success('保存成功')">Success Toast</BaseButton><BaseButton @click="toast.info('设置已保存')">Info Toast</BaseButton><BaseButton @click="toast.warning('连接正在关闭')">Warning Toast</BaseButton><BaseButton @click="toast.error('连接失败',{action:{label:'重试',run:()=>toast.info('已执行重试')}})">Error Toast</BaseButton></div><BaseAlert kind="info">信息提示</BaseAlert><BaseAlert kind="success">成功提示</BaseAlert><BaseAlert kind="warning">风险提示</BaseAlert><BaseAlert>错误提示</BaseAlert></section>
    <BaseDialog :open="dialog" title="键盘焦点验收" :busy="busy" @close="dialog=false"><BaseInput v-model="text" label="首个输入" /><BaseCheckbox v-model="busy" label="Busy：Esc 与外部点击不关闭" /><BaseButton disabled>对话框禁用按钮</BaseButton><template #footer><BaseButton :disabled="busy" @click="dialog=false">取消</BaseButton><BaseInput v-model="text" label="末尾焦点输入" /></template></BaseDialog>
    <BaseAlertDialog :open="alert" title="危险操作确认" @close="alert=false"><p>示例确认，不会删除资料。初始焦点应在取消按钮。</p><template #footer><BaseButton data-dialog-cancel @click="alert=false">取消操作</BaseButton><BaseButton variant="danger" @click="alert=false;toast.success('已确认示例')">确认操作</BaseButton></template></BaseAlertDialog>
    <BaseToastViewport :queue="toast" />
  </main>
</template>
<style scoped>
.interaction-harness { max-width: 960px; padding: 24px; margin: auto; }
.interaction-harness section { margin: 24px 0; display: grid; gap: 12px; }
.interaction-row { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
.interaction-context { padding: 12px; color: var(--color-text-primary); background: var(--color-bg-subtle); border: 1px dashed var(--color-border-hover); border-radius: var(--radius-md); }
</style>
