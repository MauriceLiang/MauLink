<script setup lang="ts">
import BaseTooltip from "../components/base/BaseTooltip.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from "reka-ui";
import { computed, reactive, ref, useId, watch } from "vue";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerAppearance } from "../../../contracts/v1/ServerAppearance";
import type { ServerAppearanceUpdate } from "../../../contracts/v1/ServerAppearanceUpdate";
import type { ServerEnvironment } from "../../../contracts/v1/ServerEnvironment";
import type { TerminalAppearanceSettings } from "../../../contracts/v1/TerminalAppearanceSettings";
import type { TerminalBackgroundFit } from "../../../contracts/v1/TerminalBackgroundFit";
import type { TerminalBackgroundPosition } from "../../../contracts/v1/TerminalBackgroundPosition";
import type { TerminalBackgroundOverlayKind } from "../../../contracts/v1/TerminalBackgroundOverlayKind";
import type { TerminalThemeMode } from "../../../contracts/v1/TerminalThemeMode";
import type { SelectedLocalFile } from "../../../contracts/v1/SelectedLocalFile";
import type { CredentialUpdate } from "../../../contracts/v1/CredentialUpdate";
import type { AppError } from "../../../contracts/v1/AppError";
import type { ServerStore } from "../stores/servers";
import type { ConnectionStore } from "../stores/connections";
import type { ServerAppearanceStore } from "../stores/server-appearance";
import type { BackgroundImagesApi } from "../ipc/background-images";
import { isFinished } from "../stores/connections";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import { credentialValidation, newServerDraft, normalizeServerDraft, validateServerDraft } from "./server-form";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseInput from "../components/base/BaseInput.vue";
import BaseSwitch from "../components/base/BaseSwitch.vue";
import type { ConnectionTestToastResult } from "../app/connection-test-toast";

const props = withDefaults(defineProps<{ open: boolean; serverId: string | null; store: ServerStore; appearanceStore: ServerAppearanceStore; backgroundImages: BackgroundImagesApi; connectionStore?: ConnectionStore; groupId?: string | null; language?: Language }>(), { groupId: null });
const emit = defineEmits<{ close: []; saved: [message: string]; testResult: [result: ConnectionTestToastResult] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const formId = useId();
const current = ref<ServerProfile | null>(null);
const draft = reactive(newServerDraft());
const secret = ref("");
const visible = ref(false);
const key = ref<SelectedLocalFile | null>(null);
const mode = ref<CredentialUpdate["mode"]>("keep");
type ServerDialogSection = "basic" | "advanced" | "appearance";
const section = ref<ServerDialogSection>("basic");
const errorSection = ref<ServerDialogSection>("basic");
const phase = ref<"loading" | "editing" | "validation" | "saving" | "error">("editing");
const selectingKey = ref(false);
const testRunning = ref(false);
const testConnectionId = ref<string | null>(null);
const error = ref("");
const failure = ref<AppError | null>(null);
const appearanceBusy = ref(false);
const appearanceError = ref("");
const appearanceRevision = ref(0);
const appearanceDraft = reactive({
  labelColorEnabled: false,
  labelColor: "#3B82F6",
  environment: null as ServerEnvironment | null,
  terminalOverrideEnabled: false,
  terminalAppearance: {
    themeMode: "followApp" as TerminalThemeMode,
    customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" },
    backgroundImage: { imageId: null as string | null, fit: "cover" as TerminalBackgroundFit, position: "center" as TerminalBackgroundPosition, imageOpacity: 100, overlayKind: "dark" as TerminalBackgroundOverlayKind, overlayOpacity: 45, blurPx: 0 },
  } satisfies TerminalAppearanceSettings,
});
const groups = props.store.groups;
const busy = computed(() => phase.value === "loading" || phase.value === "saving" || selectingKey.value || appearanceBusy.value);
const formBusy = computed(() => busy.value || testRunning.value);
let testStartedAt = 0;
let challengeStartedAt: number | null = null;
let challengeDurationMs = 0;
let testRunGeneration = 0;
let cancelTestRequested = false;

async function initialize() {
  error.value = "";
  failure.value = null;
  appearanceBusy.value = false;
  appearanceError.value = "";
  appearanceRevision.value = 0;
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
    const appearance = await props.appearanceStore.get(profile.id);
    if (!props.open) return;
    loadAppearanceDraft(appearance);
    phase.value = "editing";
  } catch (reason) {
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  }
}

function loadAppearanceDraft(appearance: ServerAppearance) {
  appearanceRevision.value = appearance.revision;
  appearanceDraft.labelColorEnabled = appearance.labelColor !== null;
  appearanceDraft.labelColor = appearance.labelColor ?? "#3B82F6";
  appearanceDraft.environment = appearance.environment;
  appearanceDraft.terminalOverrideEnabled = appearance.terminalOverrideEnabled;
  appearanceDraft.terminalAppearance = structuredClone(appearance.terminalAppearance);
}

async function chooseAppearanceImage() {
  if (busy.value) return;
  appearanceError.value = "";
  try {
    const asset = await props.backgroundImages.select();
    if (asset) appearanceDraft.terminalAppearance.backgroundImage.imageId = asset.id;
  } catch (reason) {
    appearanceError.value = presentError(mapError(reason), props.language).message;
  }
}

async function saveAppearance() {
  if (!current.value || formBusy.value || appearanceBusy.value) return;
  if (appearanceDraft.terminalAppearance.themeMode === "image" && !appearanceDraft.terminalAppearance.backgroundImage.imageId) {
    appearanceError.value = t("imageRequired");
    return;
  }
  appearanceBusy.value = true;
  appearanceError.value = "";
  const payload: ServerAppearanceUpdate = {
    serverId: current.value.id,
    expectedRevision: appearanceRevision.value,
    labelColor: appearanceDraft.labelColorEnabled ? appearanceDraft.labelColor : null,
    environment: appearanceDraft.environment,
    terminalOverrideEnabled: appearanceDraft.terminalOverrideEnabled,
    terminalAppearance: {
      themeMode: appearanceDraft.terminalAppearance.themeMode,
      customColors: { ...appearanceDraft.terminalAppearance.customColors },
      backgroundImage: { ...appearanceDraft.terminalAppearance.backgroundImage },
    },
  };
  try {
    const saved = await props.appearanceStore.update(payload);
    loadAppearanceDraft(saved);
    emit("saved", t("appearanceSaved"));
  } catch (reason) {
    appearanceError.value = presentError(mapError(reason), props.language).message;
  } finally {
    appearanceBusy.value = false;
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
    emit("testResult", { kind: "success", title: t("testConnectionSuccessTitle"), description: t("testConnectionSucceeded").replace("{latency}", String(latency)) });
    finishTest();
  } else if (snapshot.state === "failed") {
    const reason = snapshot.error ? presentError(snapshot.error, props.language).message : t("testConnectionFailedUnknown");
    emit("testResult", { kind: "error", title: t("testConnectionFailureTitle"), description: reason });
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
    emit("testResult", { kind: "error", title: t("testConnectionFailureTitle"), description: t(invalid) });
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
        emit("testResult", { kind: "error", title: t("testConnectionFailureTitle"), description: error.value });
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
    emit("testResult", { kind: "error", title: t("testConnectionFailureTitle"), description: error.value });
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
  <BaseDialog :open="open" :title="t(serverId ? 'edit' : 'add')" :busy="busy" :close-label="t('cancel')" initial-focus="input" panel-class="server-dialog" @close="closeDialog">
    <div v-if="phase === 'loading'" class="server-dialog-loading" role="status">{{ t('loading') }}</div>
    <TabsRoot v-else v-model="section" class="server-dialog-layout">
      <TabsList class="server-dialog-tabs" :aria-label="t('serverConfigSections')">
        <TabsTrigger value="basic" class="server-dialog-tab" :disabled="formBusy">{{ t('basicInfo') }}</TabsTrigger>
        <TabsTrigger value="advanced" class="server-dialog-tab" :disabled="formBusy">{{ t('advancedConfig') }}</TabsTrigger>
        <TabsTrigger v-if="serverId" value="appearance" class="server-dialog-tab" :disabled="formBusy">{{ t('appearance') }}</TabsTrigger>
      </TabsList>
      <form :id="formId" class="server-dialog-form" :data-state="phase" novalidate @submit.prevent="save">
        <TabsContent value="basic" class="server-dialog-panel">
          <fieldset :disabled="formBusy || (!!serverId && !current)" class="server-form-grid">
            <div class="server-field-full"><BaseInput v-model="draft.name!" :label="t('name')" maxlength="128" placeholder="Production Web" /></div>
            <div class="server-field-full server-host-port-row">
              <BaseInput v-model="draft.host" :label="t('host')" autocomplete="off" placeholder="192.168.1.10" />
              <div class="base-field server-port-field"><input v-model.number="draft.port" class="base-input" type="number" min="1" max="65535" required :aria-label="t('port')" /></div>
            </div>
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
              <span class="server-key-picker-label">{{ t('privateKey') }}</span>
              <div class="server-key-picker-actions">
                <BaseButton type="button" @click="selectKey">{{ t('selectKey') }}</BaseButton>
                <BaseTooltip :label="t('keyNote')"><button type="button" class="server-info-button" :aria-label="t('keyHelp')"><BaseIcon name="info" /></button></BaseTooltip>
              </div>
              <span class="server-key-picker-value">{{ key?.displayName ?? t(current?.authType === 'privateKey' && current.hasPrivateKey ? 'keepKey' : 'noKey') }}</span>
            </div>
            <div class="server-field-full server-credential-storage-row">
              <BaseSwitch v-model="draft.requireAuthentication" :label="t('requireAuthenticationOnConnect')" :disabled="busy" />
              <BaseTooltip :label="t('credentialStorageModeNote')"><button type="button" class="server-info-button" :aria-label="t('credentialStorageHelp')"><BaseIcon name="info" /></button></BaseTooltip>
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
                <div class="base-field server-port-field"><input v-model.number="draft.jumpPort" class="base-input" type="number" min="1" max="65535" :aria-label="t('jumpPort')" /></div>
              </div>
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('proxySettings') }}</h3></header>
              <BaseSelect v-model="draft.proxyType" :label="t('proxy')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('none')},{value:'socks5',label:'SOCKS5'},{value:'httpConnect',label:'HTTP CONNECT'}]" />
              <div v-if="draft.proxyType" class="server-advanced-row server-dependent-fields">
                <BaseInput v-model="draft.proxyHost!" :label="t('proxyHost')" />
                <label class="base-field server-port-field"><span>{{ t('proxyPort') }}</span><input v-model.number="draft.proxyPort" class="base-input" type="number" min="1" max="65535" /></label>
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
        <TabsContent v-if="serverId" value="appearance" class="server-dialog-panel">
          <fieldset :disabled="formBusy || !current" class="server-advanced-sections">
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('serverLabelColor') }}</h3></header>
              <label class="server-appearance-toggle"><input v-model="appearanceDraft.labelColorEnabled" type="checkbox" /> {{ t('customLabelColor') }}</label>
              <input v-if="appearanceDraft.labelColorEnabled" v-model="appearanceDraft.labelColor" class="server-appearance-color" type="color" :aria-label="t('serverLabelColor')" />
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('environment') }}</h3></header>
              <BaseSelect v-model="appearanceDraft.environment" :label="t('environment')" :options="[{value:null,label:t('noEnvironment')},{value:'production',label:t('production')},{value:'staging',label:t('staging')},{value:'development',label:t('development')},{value:'custom',label:t('customEnvironment')}]" />
            </section>
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('terminalAppearance') }}</h3></header>
              <label class="server-appearance-toggle"><input v-model="appearanceDraft.terminalOverrideEnabled" type="checkbox" /> {{ t('useServerAppearance') }}</label>
              <template v-if="appearanceDraft.terminalOverrideEnabled">
                <BaseSelect v-model="appearanceDraft.terminalAppearance.themeMode" :label="t('themeMode')" :options="[{value:'followApp',label:t('themeFollowApp')},{value:'light',label:t('themeLight')},{value:'dark',label:t('themeDark')},{value:'customColor',label:t('themeCustom')},{value:'image',label:t('themeImage')}]" />
                <div class="server-appearance-colors">
                  <label><span>{{ t('terminalBackground') }}</span><input v-model="appearanceDraft.terminalAppearance.customColors.background" type="color" /></label>
                  <label><span>{{ t('terminalForeground') }}</span><input v-model="appearanceDraft.terminalAppearance.customColors.foreground" type="color" /></label>
                  <label><span>{{ t('terminalCursor') }}</span><input v-model="appearanceDraft.terminalAppearance.customColors.cursor" type="color" /></label>
                  <label><span>{{ t('terminalSelection') }}</span><input v-model="appearanceDraft.terminalAppearance.customColors.selection" type="color" /></label>
                </div>
                <section class="server-appearance-image">
                  <h4>{{ t('backgroundImage') }}</h4>
                  <span>{{ appearanceDraft.terminalAppearance.backgroundImage.imageId ? t('imageSelected') : t('noImageSelected') }}</span>
                  <BaseButton type="button" @click="chooseAppearanceImage">{{ t('selectImage') }}</BaseButton>
                  <BaseButton v-if="appearanceDraft.terminalAppearance.backgroundImage.imageId" type="button" @click="appearanceDraft.terminalAppearance.backgroundImage.imageId = null">{{ t('clearImage') }}</BaseButton>
                  <label>{{ t('imageFit') }}<select v-model="appearanceDraft.terminalAppearance.backgroundImage.fit"><option value="cover">Cover</option><option value="contain">Contain</option><option value="stretch">Stretch</option><option value="original">Original</option><option value="tile">Tile</option></select></label>
                  <label>{{ t('imagePosition') }}<select v-model="appearanceDraft.terminalAppearance.backgroundImage.position"><option value="center">Center</option><option value="top">Top</option><option value="bottom">Bottom</option><option value="left">Left</option><option value="right">Right</option><option value="topLeft">Top left</option><option value="topRight">Top right</option><option value="bottomLeft">Bottom left</option><option value="bottomRight">Bottom right</option></select></label>
                  <label>{{ t('imageOpacity') }} <input v-model.number="appearanceDraft.terminalAppearance.backgroundImage.imageOpacity" type="range" min="10" max="100" /> {{ appearanceDraft.terminalAppearance.backgroundImage.imageOpacity }}%</label>
                  <label>{{ t('overlay') }}<select v-model="appearanceDraft.terminalAppearance.backgroundImage.overlayKind"><option value="dark">{{ t('dark') }}</option><option value="light">{{ t('light') }}</option></select></label>
                  <label>{{ t('overlayOpacity') }} <input v-model.number="appearanceDraft.terminalAppearance.backgroundImage.overlayOpacity" type="range" min="0" max="90" /> {{ appearanceDraft.terminalAppearance.backgroundImage.overlayOpacity }}%</label>
                  <label>{{ t('blur') }} <input v-model.number="appearanceDraft.terminalAppearance.backgroundImage.blurPx" type="range" min="0" max="16" /> {{ appearanceDraft.terminalAppearance.backgroundImage.blurPx }} px</label>
                </section>
              </template>
            </section>
          </fieldset>
          <p v-if="appearanceError" role="alert" class="server-form-error">{{ appearanceError }}</p>
          <BaseButton type="button" :disabled="formBusy || appearanceBusy || !current" :loading="appearanceBusy" @click="saveAppearance">{{ t('saveAppearance') }}</BaseButton>
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
