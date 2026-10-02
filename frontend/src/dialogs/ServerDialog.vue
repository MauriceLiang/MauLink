<script setup lang="ts">
import BaseTooltip from "../components/base/BaseTooltip.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from "reka-ui";
import { computed, reactive, ref, useId, watch } from "vue";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { SelectedLocalFile } from "../../../contracts/v1/SelectedLocalFile";
import type { CredentialUpdate } from "../../../contracts/v1/CredentialUpdate";
import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerStore } from "../stores/servers";
import type { ConnectionStore } from "../stores/connections";
import { isFinished } from "../stores/connections";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { credentialValidation, newServerDraft, normalizeServerDraft, validateServerDraft } from "./server-form";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseInput from "../components/base/BaseInput.vue";

const props = withDefaults(defineProps<{ open: boolean; serverId: string | null; store: ServerStore; connectionStore?: ConnectionStore; groupId?: string | null; language?: Language }>(), { groupId: null });
const emit = defineEmits<{ close: []; saved: [message: string]; testResult: [result: { kind: "success" | "error"; message: string }] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const formId = useId();
const current = ref<ServerProfile | null>(null);
const draft = reactive(newServerDraft());
const secret = ref("");
const visible = ref(false);
const key = ref<SelectedLocalFile | null>(null);
const mode = ref<CredentialUpdate["mode"]>("keep");
type ServerDialogSection = "basic" | "advanced";
const section = ref<ServerDialogSection>("basic");
const errorSection = ref<ServerDialogSection>("basic");
const phase = ref<"loading" | "editing" | "validation" | "saving" | "error">("editing");
const selectingKey = ref(false);
const testRunning = ref(false);
const testConnectionId = ref<string | null>(null);
const error = ref("");
const failure = ref<AppError | null>(null);
const groups = props.store.groups;
const busy = computed(() => phase.value === "loading" || phase.value === "saving" || selectingKey.value);
const formBusy = computed(() => busy.value || testRunning.value);
let testStartedAt = 0;
let challengeStartedAt: number | null = null;
let challengeDurationMs = 0;
let testRunGeneration = 0;
let cancelTestRequested = false;

async function initialize() {
  error.value = "";
  failure.value = null;
  secret.value = "";
  key.value = null;
  visible.value = false;
  section.value = "basic";
  errorSection.value = "basic";
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

const validPort = (value: number | null) => Number.isInteger(value) && Number(value) >= 1 && Number(value) <= 65535;
function validationSection(invalid: ServerMessage): ServerDialogSection {
  if (["keepaliveInvalid", "timeoutInvalid", "jumpInvalid", "proxyInvalid"].includes(invalid)) return "advanced";
  if (invalid === "portInvalid") return validPort(draft.port) ? "advanced" : "basic";
  return "basic";
}

watch(() => props.open, open => {
  testRunGeneration++;
  if (open) {
    cancelTestRequested = false;
    void initialize();
  } else {
    cancelTestRequested = true;
    void cancelTest();
    secret.value = "";
    key.value = null;
  }
}, { immediate: true });
watch(() => draft.authType, () => { key.value = null; secret.value = ""; visible.value = false; });
watch(mode, () => { secret.value = ""; visible.value = false; });

watch(() => {
  const id = testConnectionId.value;
  return id ? props.connectionStore?.snapshots.value[id] ?? null : null;
}, snapshot => {
  if (!testRunning.value || !snapshot) return;
  const now = performance.now();
  const awaitingUser = snapshot.state === "awaitingHostTrust" || snapshot.state === "awaitingCredentials";
  if (awaitingUser && challengeStartedAt === null) challengeStartedAt = now;
  else if (!awaitingUser && challengeStartedAt !== null) {
    challengeDurationMs += now - challengeStartedAt;
    challengeStartedAt = null;
  }
  if (snapshot.state === "closed") {
    const latency = Math.max(0, Math.round(now - testStartedAt - challengeDurationMs));
    emit("testResult", { kind: "success", message: t("testConnectionSucceeded").replace("{latency}", String(latency)) });
    finishTest();
  } else if (snapshot.state === "failed") {
    const reason = snapshot.error ? presentError(snapshot.error, props.language).message : t("testConnectionFailedUnknown");
    emit("testResult", { kind: "error", message: `${t("testConnectionFailed")}${reason}` });
    finishTest();
  } else if (snapshot.state === "cancelled") finishTest();
}, { flush: "sync" });

function finishTest() {
  const id = testConnectionId.value;
  testRunning.value = false;
  challengeStartedAt = null;
  testConnectionId.value = null;
  if (id) props.connectionStore?.dismiss(id);
}

async function cancelTest() {
  const id = testConnectionId.value;
  testRunning.value = false;
  challengeStartedAt = null;
  testConnectionId.value = null;
  if (!id || !props.connectionStore) return;
  const snapshot = props.connectionStore.snapshots.value[id];
  if (snapshot && !isFinished(snapshot) && snapshot.state !== "ready") await props.connectionStore.cancel(id);
  const finalSnapshot = props.connectionStore.snapshots.value[id];
  if (finalSnapshot && isFinished(finalSnapshot)) props.connectionStore.dismiss(id);
}

function closeDialog() {
  cancelTestRequested = true;
  testRunGeneration++;
  void cancelTest();
  emit("close");
}

async function testConnection() {
  if (!props.connectionStore || formBusy.value) return;
  const profile = normalizeServerDraft(draft, key.value);
  const invalid = validateServerDraft(profile, current.value, key.value)
    ?? (current.value && mode.value === "keep" ? credentialValidation(current.value, profile, { mode: "keep" }) : null);
  if (invalid) {
    section.value = "basic";
    errorSection.value = "basic";
    error.value = t(invalid);
    emit("testResult", { kind: "error", message: `${t("testConnectionFailed")}${t(invalid)}` });
    return;
  }
  error.value = "";
  failure.value = null;
  cancelTestRequested = false;
  const generation = ++testRunGeneration;
  testStartedAt = performance.now();
  challengeStartedAt = null;
  challengeDurationMs = 0;
  testRunning.value = true;
  try {
    const savedProfile = current.value && !key.value
      && (current.value.hasPrivateKey || (mode.value === "keep" && current.value.hasSavedCredential))
      ? {
          serverId: current.value.id,
          expectedRevision: current.value.revision,
          useSavedCredential: mode.value === "keep" && current.value.hasSavedCredential,
        }
      : undefined;
    const credential = !current.value || mode.value === "replace" ? secret.value || null : null;
    const snapshot = await props.connectionStore.startDraftTest(profile, credential, savedProfile);
    if (!snapshot) {
      if (generation === testRunGeneration) {
        error.value = t("testConnectionUnavailable");
        testRunning.value = false;
        emit("testResult", { kind: "error", message: `${t("testConnectionFailed")}${error.value}` });
      }
      return;
    }
    if (generation !== testRunGeneration || cancelTestRequested || !props.open) {
      await props.connectionStore.cancel(snapshot.connectionId);
      const current = props.connectionStore.snapshots.value[snapshot.connectionId];
      if (current && isFinished(current)) props.connectionStore.dismiss(snapshot.connectionId);
      return;
    }
    testConnectionId.value = snapshot.connectionId;
  } catch (reason) {
    if (generation !== testRunGeneration) return;
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    errorSection.value = "basic";
    testRunning.value = false;
    emit("testResult", { kind: "error", message: `${t("testConnectionFailed")}${error.value}` });
  }
}

async function selectKey() {
  if (busy.value) return;
  selectingKey.value = true;
  errorSection.value = "basic";
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
  if (formBusy.value || (props.serverId && !current.value)) return;
  failure.value = null;
  error.value = "";
  const profile = normalizeServerDraft(draft, key.value);
  const credential: CredentialUpdate = current.value
    ? mode.value === "replace" ? { mode: "replace", secret: secret.value } : { mode: mode.value }
    : secret.value ? { mode: "replace", secret: secret.value } : { mode: "clear" };
  const invalid = validateServerDraft(profile, current.value, key.value) ?? credentialValidation(current.value, profile, credential);
  if (invalid) {
    section.value = validationSection(invalid);
    errorSection.value = section.value;
    phase.value = "validation";
    error.value = t(invalid);
    return;
  }
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
    if (failure.value.code === "REVISION_CONFLICT") section.value = "basic";
    errorSection.value = failure.value.code === "REVISION_CONFLICT" ? "basic" : section.value;
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  }
}
</script>

<template>
  <BaseDialog :open="open" :title="t(serverId ? 'edit' : 'add')" :busy="busy" :close-label="t('cancel')" panel-class="server-dialog" @close="closeDialog">
    <div v-if="phase === 'loading'" class="server-dialog-loading" role="status">{{ t('loading') }}</div>
    <TabsRoot v-else v-model="section" class="server-dialog-layout">
      <TabsList class="server-dialog-tabs" :aria-label="t('serverConfigSections')">
        <TabsTrigger value="basic" class="server-dialog-tab" :disabled="formBusy">{{ t('basicInfo') }}</TabsTrigger>
        <TabsTrigger value="advanced" class="server-dialog-tab" :disabled="formBusy">{{ t('advancedConfig') }}</TabsTrigger>
      </TabsList>
      <form :id="formId" class="server-dialog-form" :data-state="phase" novalidate @submit.prevent="save">
        <TabsContent value="basic" class="server-dialog-panel">
          <fieldset :disabled="formBusy || (!!serverId && !current)" class="server-form-grid">
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
          </fieldset>
          <p v-if="error && errorSection === 'basic'" role="alert" class="server-form-error">{{ error }}</p>
          <BaseButton v-if="failure?.code === 'REVISION_CONFLICT' || (serverId && !current)" :disabled="busy" @click="initialize">{{ t(current ? 'reload' : 'retry') }}</BaseButton>
        </TabsContent>
        <TabsContent value="advanced" class="server-dialog-panel">
          <fieldset :disabled="formBusy || (!!serverId && !current)" class="server-advanced-sections">
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('organization') }}</h3></header>
              <BaseSelect v-model="draft.groupId" :label="t('group')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('ungrouped')},...groups.map(group => ({value:group.id,label:group.name}))]" />
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('jumpRouting') }}</h3></header>
              <div class="server-advanced-row">
                <BaseInput v-model="draft.jumpHost!" :label="t('jumpHost')" placeholder="user@bastion.example.com" />
                <label class="base-field"><span>{{ t('jumpPort') }}</span><input v-model.number="draft.jumpPort" class="base-input" type="number" min="1" max="65535" /></label>
              </div>
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('proxySettings') }}</h3></header>
              <BaseSelect v-model="draft.proxyType" :label="t('proxy')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('none')},{value:'socks5',label:'SOCKS5'},{value:'httpConnect',label:'HTTP CONNECT'}]" />
              <div v-if="draft.proxyType" class="server-advanced-row server-dependent-fields">
                <BaseInput v-model="draft.proxyHost!" :label="t('proxyHost')" />
                <label class="base-field"><span>{{ t('proxyPort') }}</span><input v-model.number="draft.proxyPort" class="base-input" type="number" min="1" max="65535" /></label>
              </div>
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('connectionSettings') }}</h3></header>
              <div class="server-connection-settings">
                <label class="base-field"><span>{{ t('keepalive') }}</span><input v-model.number="draft.keepaliveIntervalSeconds" class="base-input" type="number" min="5" max="300" /></label>
                <label class="base-field"><span>{{ t('timeout') }}</span><input v-model.number="draft.connectTimeoutMs" class="base-input" type="number" min="1000" max="120000" /></label>
              </div>
            </section>
          </fieldset>
          <p v-if="error && errorSection === 'advanced'" role="alert" class="server-form-error">{{ error }}</p>
        </TabsContent>
      </form>
    </TabsRoot>
    <template #footer>
      <div class="server-dialog-footer-leading">
        <BaseButton v-if="connectionStore" :disabled="busy" :loading="testRunning" @click="testConnection">{{ t(testRunning ? 'testingConnection' : 'testConnection') }}</BaseButton>
      </div>
      <div class="server-dialog-footer-actions">
        <BaseButton :disabled="busy" @click="closeDialog">{{ t('cancel') }}</BaseButton>
        <BaseButton variant="primary" type="submit" :form="formId" :disabled="formBusy || (!!serverId && !current)" :loading="phase === 'saving'">{{ t(phase === 'saving' ? 'saving' : 'save') }}</BaseButton>
      </div>
    </template>
  </BaseDialog>
</template>
