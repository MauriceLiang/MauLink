<script setup lang="ts">
import BaseTooltip from "../components/base/BaseTooltip.vue";
import BaseCollapsible from "../components/base/BaseCollapsible.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { computed, reactive, ref, useId, watch } from "vue";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { SelectedLocalFile } from "../../../contracts/v1/SelectedLocalFile";
import type { CredentialUpdate } from "../../../contracts/v1/CredentialUpdate";
import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerStore } from "../stores/servers";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { credentialValidation, newServerDraft, normalizeServerDraft, validateServerDraft } from "./server-form";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseInput from "../components/base/BaseInput.vue";

const props = withDefaults(defineProps<{ open: boolean; serverId: string | null; store: ServerStore; groupId?: string | null; language?: Language }>(), { groupId: null });
const emit = defineEmits<{ close: []; saved: [message: string] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const formId = useId();
const current = ref<ServerProfile | null>(null);
const draft = reactive(newServerDraft());
const secret = ref("");
const visible = ref(false);
const key = ref<SelectedLocalFile | null>(null);
const mode = ref<CredentialUpdate["mode"]>("keep");
const advanced = ref(false);
const phase = ref<"loading" | "editing" | "validation" | "saving" | "error">("editing");
const selectingKey = ref(false);
const error = ref("");
const failure = ref<AppError | null>(null);
const groups = props.store.groups;
const busy = computed(() => phase.value === "loading" || phase.value === "saving" || selectingKey.value);

async function initialize() {
  error.value = "";
  failure.value = null;
  secret.value = "";
  key.value = null;
  visible.value = false;
  advanced.value = false;
  current.value = null;
  mode.value = "keep";
  Object.assign(draft, newServerDraft(null, props.groupId));
  if (!props.serverId) { phase.value = "editing"; return; }
  phase.value = "loading";
  try {
    const profile = await props.store.getServer(props.serverId);
    if (!props.open) return;
    current.value = profile;
    Object.assign(draft, newServerDraft(profile));
    phase.value = "editing";
  } catch (reason) {
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  }
}
watch(() => props.open, open => {
  if (open) void initialize();
  else { secret.value = ""; key.value = null; }
}, { immediate: true });
watch(() => draft.authType, () => { key.value = null; secret.value = ""; visible.value = false; });
watch(mode, () => { secret.value = ""; visible.value = false; });

async function selectKey() {
  if (busy.value) return;
  selectingKey.value = true;
  error.value = "";
  failure.value = null;
  try {
    const selected = await props.store.selectPrivateKey();
    if (selected) key.value = selected;
  } catch (reason) {
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  } finally { selectingKey.value = false; }
}

async function save() {
  if (busy.value || (props.serverId && !current.value)) return;
  failure.value = null;
  error.value = "";
  const profile = normalizeServerDraft(draft, key.value);
  const credential: CredentialUpdate = current.value
    ? mode.value === "replace" ? { mode: "replace", secret: secret.value } : { mode: mode.value }
    : secret.value ? { mode: "replace", secret: secret.value } : { mode: "clear" };
  const invalid = validateServerDraft(profile, current.value, key.value) ?? credentialValidation(current.value, profile, credential);
  if (invalid) { phase.value = "validation"; error.value = t(invalid); return; }
  phase.value = "saving";
  try {
    const result = current.value
      ? await props.store.update({ serverId: current.value.id, expectedRevision: current.value.revision, profile, credential })
      : await props.store.create({ profile, credential });
    secret.value = "";
    key.value = null;
    emit("saved", result.credentialCleanupPending ? t("cleanup") : current.value ? t("updated") : t("added"));
    emit("close");
  } catch (reason) {
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  }
}
</script>

<template>
  <BaseDialog :open="open" :title="t(serverId ? 'edit' : 'add')" :busy="busy" :close-label="t('cancel')" panel-class="server-dialog" @close="$emit('close')">
    <p v-if="phase === 'loading'" role="status">{{ t('loading') }}</p>
    <form v-else :id="formId" class="server-dialog-form" :data-state="phase" novalidate @submit.prevent="save">
      <fieldset :disabled="busy || (!!serverId && !current)" class="server-form-grid">
        <div class="server-field-full"><BaseInput v-model="draft.name!" :label="t('name')" maxlength="128" placeholder="Production Web" /></div>
        <BaseInput v-model="draft.host" :label="t('host')" autocomplete="off" placeholder="192.168.1.10" />
        <label class="base-field"><span>{{ t('port') }}</span><input v-model.number="draft.port" class="base-input" type="number" min="1" max="65535" required /></label>
        <div class="server-field-full"><BaseInput v-model="draft.username" :label="t('username')" autocomplete="username" placeholder="root" /></div>
        <div class="server-field-full base-field"><span>{{ t('auth') }}</span><div class="server-auth-options" role="group" :aria-label="t('auth')">
          <BaseButton :aria-pressed="draft.authType === 'password'" @click="draft.authType = 'password'">{{ t('password') }}</BaseButton>
          <BaseButton :aria-pressed="draft.authType === 'privateKey'" @click="draft.authType = 'privateKey'">{{ t('key') }}</BaseButton>
        </div></div>
        <BaseSelect v-if="current" v-model="mode" class="server-field-full" :label="t('credential')" :disabled="busy" :options="[{value:'keep',label:t('keep')},{value:'replace',label:t('replace')},{value:'clear',label:t('clear')}]" />
        <div v-if="!current || mode === 'replace'" class="server-field-full server-secret-field">
          <BaseInput v-model="secret" :label="t(draft.authType === 'privateKey' ? 'passphrase' : 'password')" :type="visible ? 'text' : 'password'" autocomplete="new-password" />
          <BaseTooltip :label="visible ? '隐藏凭据 / Hide credential' : '显示凭据 / Show credential'"><button type="button" class="server-secret-toggle" :aria-pressed="visible" :aria-label="visible ? '隐藏凭据 / Hide credential' : '显示凭据 / Show credential'" @click="visible = !visible"><BaseIcon :name="visible ? 'eye-off' : 'eye'" /></button></BaseTooltip>
          <p class="server-form-note">{{ t('secretNote') }}</p>
        </div>
        <div v-if="draft.authType === 'privateKey'" class="server-field-full server-key-picker">
          <span>{{ t('privateKey') }}</span><p class="server-form-note">{{ t('keyNote') }}</p>
          <div><span>{{ key?.displayName ?? t(current?.authType === 'privateKey' && current.hasPrivateKey ? 'keepKey' : 'noKey') }}</span><BaseButton @click="selectKey">{{ t('selectKey') }}</BaseButton></div>
        </div>
        <BaseCollapsible v-model:open="advanced" class="server-field-full" :label="t('advanced')" trigger-class="server-advanced-toggle" :disabled="busy || (!!serverId && !current)">
        <div class="server-advanced-grid">
          <BaseSelect v-model="draft.groupId" :label="t('group')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('ungrouped')},...groups.map(group => ({value:group.id,label:group.name}))]" />
          <BaseInput v-model="draft.jumpHost!" :label="t('jumpHost')" placeholder="user@bastion.example.com" />
          <label class="base-field"><span>{{ t('jumpPort') }}</span><input v-model.number="draft.jumpPort" class="base-input" type="number" min="1" max="65535" /></label>
          <BaseSelect v-model="draft.proxyType" :label="t('proxy')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('none')},{value:'socks5',label:'SOCKS5'},{value:'httpConnect',label:'HTTP CONNECT'}]" />
          <template v-if="draft.proxyType"><BaseInput v-model="draft.proxyHost!" :label="t('proxyHost')" /><label class="base-field"><span>{{ t('proxyPort') }}</span><input v-model.number="draft.proxyPort" class="base-input" type="number" min="1" max="65535" /></label></template>
          <label class="base-field"><span>{{ t('keepalive') }}</span><input v-model.number="draft.keepaliveIntervalSeconds" class="base-input" type="number" min="5" max="300" /></label>
          <label class="base-field"><span>{{ t('timeout') }}</span><input v-model.number="draft.connectTimeoutMs" class="base-input" type="number" min="1000" max="120000" /></label>
        </div>
        </BaseCollapsible>
      </fieldset>
      <p v-if="error" role="alert" class="server-form-error">{{ error }}</p>
      <BaseButton v-if="failure?.code === 'REVISION_CONFLICT' || (serverId && !current)" :disabled="busy" @click="initialize">{{ t(current ? 'reload' : 'retry') }}</BaseButton>
    </form>
    <template #footer><BaseButton :disabled="busy" @click="$emit('close')">{{ t('cancel') }}</BaseButton><BaseButton variant="primary" type="submit" :form="formId" :disabled="!!serverId && !current" :loading="busy">{{ t(phase === 'saving' ? 'saving' : 'save') }}</BaseButton></template>
  </BaseDialog>
</template>
