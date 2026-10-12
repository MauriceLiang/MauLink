<script setup lang="ts">
import BaseTooltip from "../components/base/BaseTooltip.vue";
import BaseInfoPopover from "../components/base/BaseInfoPopover.vue";
import BaseIcon from "../components/base/BaseIcon.vue";
import BaseSelect from "../components/base/BaseSelect.vue";
import BaseDropdownMenu from "../components/base/BaseDropdownMenu.vue";
import BaseAlertDialog from "../components/base/BaseAlertDialog.vue";
import BaseAlert from "../components/base/BaseAlert.vue";
import type { MenuItem } from "../components/base/menu";
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from "reka-ui";
import { computed, reactive, ref, useId, watch } from "vue";
import type { Language } from "../../../contracts/v1/Language";
import type { ServerProfile } from "../../../contracts/v1/ServerProfile";
import type { ServerProfileDraft } from "../../../contracts/v1/ServerProfileDraft";
import type { AppSettings } from "../../../contracts/v1/AppSettings";
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
import ServerAppearancePanel from "./ServerAppearancePanel.vue";
import { defaultSettings } from "../terminal/preferences";
import { isFinished } from "../stores/connections";
import { serverText, type ServerMessage } from "../i18n/servers";
import { mapError } from "../errors/mapper";
import { presentError } from "../errors/presenter";
import {
  cancelReplace,
  credentialValidation,
  enterReplace,
  initialCredentialEditAction,
  markPendingClear,
  newServerDraft,
  normalizeServerDraft,
  resolveCredentialUpdate,
  undoPendingClear,
  validateServerDraft,
  type CredentialEditAction,
} from "./server-form";
import BaseButton from "../components/base/BaseButton.vue";
import BaseDialog from "../components/base/BaseDialog.vue";
import BaseInput from "../components/base/BaseInput.vue";
import type { ConnectionTestToastResult } from "../app/connection-test-toast";

const props = withDefaults(defineProps<{ open: boolean; serverId: string | null; store: ServerStore; appearanceStore: ServerAppearanceStore; backgroundImages: BackgroundImagesApi; connectionStore?: ConnectionStore; groupId?: string | null; language?: Language; globalSettings?: AppSettings }>(), { groupId: null });
const emit = defineEmits<{ close: []; saved: [message: string]; testResult: [result: ConnectionTestToastResult]; revealCredential: [serverId: string] }>();
const t = (key: ServerMessage) => serverText(key, props.language);
const formId = useId();
const globalSettings = computed(() => props.globalSettings ?? defaultSettings);
const current = ref<ServerProfile | null>(null);
const savedProfileSnapshot = ref<ServerProfile | null>(null);
const draft = reactive(newServerDraft());
const secret = ref("");
const visible = ref(false);
const key = ref<SelectedLocalFile | null>(null);
type CredentialStorageChoice = "system" | "local";
const credentialStorageChoice = computed<CredentialStorageChoice>({
  get: () => draft.requireAuthentication ? "system" : "local",
  set: choice => { draft.requireAuthentication = choice === "system"; },
});
type PrivateKeyUiState = "empty" | "selected" | "existing" | "expired";
const keyValidationNow = ref(Date.now());
function refreshKeyStatus() { keyValidationNow.value = Date.now(); }
const privateKeyUiState = computed<PrivateKeyUiState>(() => {
  if (key.value) return key.value.expiresAtMs <= keyValidationNow.value ? "expired" : "selected";
  if (current.value?.authType === "privateKey" && current.value.hasPrivateKey) return "existing";
  return "empty";
});
const privateKeyButtonText = computed(() => privateKeyUiState.value === "empty" ? t("browseKey") : privateKeyUiState.value === "expired" ? t("reselectKey") : t("replaceKey"));
const privateKeyDisplayText = computed(() => key.value?.displayName ?? (privateKeyUiState.value === "existing" ? t("configuredKey") : t("noKey")));
const credentialAction = ref<CredentialEditAction>(initialCredentialEditAction(!props.serverId));
const credentialStorageEditing = ref(false);
const removeCredentialOpen = ref(false);
type ServerDialogSection = "basic" | "advanced" | "appearance";
const section = ref<ServerDialogSection>("basic");
const errorSection = ref<ServerDialogSection>("basic");
const phase = ref<"loading" | "editing" | "validation" | "saving" | "error">("editing");
const selectingKey = ref(false);
const testRunning = ref(false);
const testConnectionId = ref<string | null>(null);
const error = ref("");
const failure = ref<AppError | null>(null);
const appearanceError = ref("");
const appearanceRevision = ref(0);
const savedAppearanceSnapshot = ref<ServerAppearance | null>(null);
const selectingAppearanceImage = ref(false);
const resolvingAppearanceImage = ref(false);
const imageBusy = computed(() => selectingAppearanceImage.value || resolvingAppearanceImage.value);
const importedImageIds = ref<string[]>([]);
const pendingImageCleanupIds = ref<string[]>([]);
const originalAppearanceImageId = ref<string | null>(null);
const appearanceImageUrl = ref("");
const appearanceImageName = ref("");
const appearanceImageError = ref("");
const globalImageUrl = ref("");
const globalImageBusy = ref(false);
const globalImageError = ref("");
const discardDialogOpen = ref(false);
const discardAction = ref<"close" | "reload">("close");
const discardBusy = ref(false);
const profileCommittedInSession = ref(false);
const appearanceDraft = reactive({
  labelColor: null as string | null,
  environment: null as ServerEnvironment | null,
  terminalOverrideEnabled: false,
  terminalAppearance: {
    themeMode: "followApp" as TerminalThemeMode,
    customColors: { background: "#111318", foreground: "#EAECF0", cursor: "#3B82F6", selection: "#3B82F6" },
    backgroundImage: { imageId: null as string | null, fit: "cover" as TerminalBackgroundFit, position: "center" as TerminalBackgroundPosition, imageOpacity: 100, overlayKind: "dark" as TerminalBackgroundOverlayKind, overlayOpacity: 45, blurPx: 0 },
  } satisfies TerminalAppearanceSettings,
});
const groups = props.store.groups;
const busy = computed(() => phase.value === "loading" || phase.value === "saving" || selectingKey.value || imageBusy.value || discardBusy.value);
const formBusy = computed(() => busy.value || testRunning.value);
const appearancePanelModel = computed(() => ({
  labelColor: appearanceDraft.labelColor,
  terminalOverrideEnabled: appearanceDraft.terminalOverrideEnabled,
  terminalAppearance: appearanceDraft.terminalAppearance,
}));
const profileDirty = computed(() => {
  const baseline = savedProfileSnapshot.value ?? newServerDraft(null, props.groupId);
  const normalized = normalizeServerDraft(draft, key.value);
  const comparable = (value: ServerProfile | ServerProfileDraft) => ({
    name: value.name?.trim() || null,
    host: value.host,
    port: value.port,
    username: value.username,
    authType: value.authType,
    requireAuthentication: value.requireAuthentication,
    groupId: value.groupId,
    connectTimeoutMs: value.connectTimeoutMs,
    keepaliveIntervalSeconds: value.keepaliveIntervalSeconds,
    jumpHost: value.jumpHost?.trim() || null,
    jumpPort: value.jumpPort,
    proxyType: value.proxyType,
    proxyHost: value.proxyHost?.trim() || null,
    proxyPort: value.proxyPort,
  });
  return JSON.stringify(comparable(normalized)) !== JSON.stringify(comparable(baseline))
    || key.value !== null
    || credentialAction.value === "pendingClear"
    || (credentialAction.value === "editing" && (!!secret.value || !!current.value?.hasSavedCredential));
});
function appearanceBusinessValue(value = appearanceDraft) {
  return {
    labelColor: value.labelColor,
    environment: value.environment,
    terminalOverrideEnabled: value.terminalOverrideEnabled,
    terminalAppearance: {
      themeMode: value.terminalAppearance.themeMode,
      customColors: { ...value.terminalAppearance.customColors },
      backgroundImage: { ...value.terminalAppearance.backgroundImage },
    },
  };
}
const appearanceDirty = computed(() => savedAppearanceSnapshot.value !== null
  && JSON.stringify(appearanceBusinessValue()) !== JSON.stringify(appearanceBusinessValue(savedAppearanceSnapshot.value)));
const hasUnsavedChanges = computed(() => profileDirty.value || appearanceDirty.value || importedImageIds.value.length > 0);
const saveDisabled = computed(() => formBusy.value || (!!props.serverId && !current.value) || (!!current.value && !hasUnsavedChanges.value));
const credentialStorageSummary = computed(() => t(draft.requireAuthentication ? "credentialStorageSystem" : "credentialStorageLocal"));
const credentialIdentityChanged = computed(() => {
  if (!current.value?.hasSavedCredential) return false;
  const profile = normalizeServerDraft(draft, key.value);
  return credentialValidation(current.value, profile, { mode: "keep" }) === "identityChanged";
});
const credentialMenuItems = computed<MenuItem[]>(() => [
  { id: "reveal", label: t(current.value?.authType === "privateKey" ? "credentialRevealPassphrase" : "credentialRevealPassword") },
  { id: "change-storage", label: t("credentialChangeStorage") },
  { id: "remove", label: t("credentialRemove"), danger: true, separatorBefore: true },
]);
let testStartedAt = 0;
let challengeStartedAt: number | null = null;
let challengeDurationMs = 0;
let testRunGeneration = 0;
let cancelTestRequested = false;
let dialogGeneration = 0;
let appearanceImageRequest = 0;
let globalImageRequest = 0;

async function initialize() {
  const generation = ++dialogGeneration;
  appearanceImageRequest++;
  refreshKeyStatus();
  error.value = "";
  failure.value = null;
  appearanceError.value = "";
  appearanceImageError.value = "";
  appearanceRevision.value = 0;
  savedAppearanceSnapshot.value = null;
  savedProfileSnapshot.value = null;
  profileCommittedInSession.value = false;
  importedImageIds.value = [];
  originalAppearanceImageId.value = null;
  appearanceImageUrl.value = "";
  appearanceImageName.value = "";
  secret.value = "";
  key.value = null;
  visible.value = false;
  credentialAction.value = initialCredentialEditAction(!props.serverId);
  credentialStorageEditing.value = false;
  removeCredentialOpen.value = false;
  section.value = "basic";
  errorSection.value = "basic";
  current.value = null;
  Object.assign(draft, newServerDraft(null, props.groupId));
  appearanceDraft.labelColor = null;
  appearanceDraft.environment = null;
  appearanceDraft.terminalOverrideEnabled = false;
  appearanceDraft.terminalAppearance = {
    themeMode: defaultSettings.terminalThemeMode,
    customColors: structuredClone(defaultSettings.terminalCustomColors),
    backgroundImage: structuredClone(defaultSettings.terminalBackgroundImage),
  };
  if (!props.serverId) { phase.value = "editing"; return; }
  phase.value = "loading";
  try {
    const profile = await props.store.getServer(props.serverId);
    if (generation !== dialogGeneration || !props.open) return;
    current.value = profile;
    savedProfileSnapshot.value = structuredClone(profile);
    Object.assign(draft, newServerDraft(profile));
    credentialAction.value = initialCredentialEditAction(false);
    const appearance = await props.appearanceStore.get(profile.id);
    if (generation !== dialogGeneration || !props.open) return;
    loadAppearanceDraft(appearance);
    phase.value = "editing";
  } catch (reason) {
    if (generation !== dialogGeneration || !props.open) return;
    failure.value = mapError(reason);
    error.value = presentError(failure.value, props.language).message;
    phase.value = "error";
  }
}

function loadAppearanceDraft(appearance: ServerAppearance) {
  savedAppearanceSnapshot.value = structuredClone(appearance);
  appearanceRevision.value = appearance.revision;
  appearanceDraft.labelColor = appearance.labelColor;
  appearanceDraft.environment = appearance.environment;
  appearanceDraft.terminalOverrideEnabled = appearance.terminalOverrideEnabled;
  appearanceDraft.terminalAppearance = structuredClone(appearance.terminalAppearance);
  originalAppearanceImageId.value = appearance.terminalAppearance.backgroundImage.imageId;
}

async function chooseAppearanceImage() {
  if (formBusy.value) return;
  appearanceError.value = "";
  appearanceImageError.value = "";
  selectingAppearanceImage.value = true;
  const generation = dialogGeneration;
  try {
    const asset = await props.backgroundImages.select();
    if (!asset) return;
    if (generation !== dialogGeneration || !props.open) {
      const cleaned = await cleanupImageIds([asset.id]);
      if (!cleaned) emit("saved", t("imageCleanupPending"));
      return;
    }
    if (!importedImageIds.value.includes(asset.id)) importedImageIds.value.push(asset.id);
    appearanceDraft.terminalAppearance.backgroundImage.imageId = asset.id;
  } catch (reason) {
    appearanceImageError.value = presentError(mapError(reason), props.language).message;
  } finally {
    selectingAppearanceImage.value = false;
  }
}

async function resolveAppearanceImage(imageId: string | null) {
  const request = ++appearanceImageRequest;
  appearanceImageUrl.value = "";
  appearanceImageName.value = "";
  appearanceImageError.value = "";
  if (!imageId) { resolvingAppearanceImage.value = false; return; }
  resolvingAppearanceImage.value = true;
  try {
    const result = await props.backgroundImages.resolve(imageId);
    if (request !== appearanceImageRequest || appearanceDraft.terminalAppearance.backgroundImage.imageId !== imageId) return;
    appearanceImageUrl.value = result.src;
    appearanceImageName.value = result.asset.fileName;
  } catch (reason) {
    if (request === appearanceImageRequest && appearanceDraft.terminalAppearance.backgroundImage.imageId === imageId) {
      appearanceImageError.value = presentError(mapError(reason), props.language).message;
    }
  } finally {
    if (request === appearanceImageRequest) resolvingAppearanceImage.value = false;
  }
}

async function resolveGlobalImage(imageId: string | null) {
  const request = ++globalImageRequest;
  globalImageUrl.value = "";
  globalImageError.value = "";
  if (!imageId) { globalImageBusy.value = false; return; }
  globalImageBusy.value = true;
  try {
    const result = await props.backgroundImages.resolve(imageId);
    if (request === globalImageRequest && globalSettings.value.terminalBackgroundImage.imageId === imageId) globalImageUrl.value = result.src;
  } catch (reason) {
    if (request === globalImageRequest && globalSettings.value.terminalBackgroundImage.imageId === imageId) globalImageError.value = presentError(mapError(reason), props.language).message;
  } finally {
    if (request === globalImageRequest) globalImageBusy.value = false;
  }
}

async function cleanupImageIds(ids: string[], keep: string | null = null) {
  const pending = new Set(pendingImageCleanupIds.value);
  for (const id of new Set(ids)) {
    if (id === keep) { pending.delete(id); continue; }
    try {
      await props.backgroundImages.delete(id);
      pending.delete(id);
    } catch (reason) {
      const code = mapError(reason).code;
      if (code === "RESOURCE_NOT_FOUND") pending.delete(id);
      else if (code === "RESOURCE_IN_USE") pending.add(id);
      else pending.add(id);
    }
  }
  pendingImageCleanupIds.value = [...pending];
  return pendingImageCleanupIds.value.length === 0;
}

async function retryPendingImageCleanup() {
  if (pendingImageCleanupIds.value.length) await cleanupImageIds([...pendingImageCleanupIds.value]);
}

function updateAppearancePanel(value: { labelColor: string | null; terminalOverrideEnabled: boolean; terminalAppearance: TerminalAppearanceSettings }) {
  appearanceDraft.labelColor = value.labelColor;
  appearanceDraft.terminalOverrideEnabled = value.terminalOverrideEnabled;
  appearanceDraft.terminalAppearance = {
    themeMode: value.terminalAppearance.themeMode,
    customColors: { ...value.terminalAppearance.customColors },
    backgroundImage: { ...value.terminalAppearance.backgroundImage },
  };
}

function removeAppearanceImage() {
  appearanceDraft.terminalAppearance.backgroundImage.imageId = null;
  appearanceImageUrl.value = "";
  appearanceImageName.value = "";
  if (appearanceDraft.terminalAppearance.themeMode === "image") appearanceDraft.terminalAppearance.themeMode = "followApp";
}

const validPort = (value: number | null) => Number.isInteger(value) && Number(value) >= 1 && Number(value) <= 65535;
function validationSection(invalid: ServerMessage): ServerDialogSection {
  if (["keepaliveInvalid", "timeoutInvalid", "jumpInvalid", "proxyInvalid"].includes(invalid)) return "advanced";
  if (invalid === "portInvalid") return validPort(draft.port) ? "advanced" : "basic";
  return "basic";
}

watch(() => appearanceDraft.terminalAppearance.backgroundImage.imageId, imageId => {
  void resolveAppearanceImage(imageId);
});
watch(() => globalSettings.value.terminalBackgroundImage.imageId, imageId => {
  void resolveGlobalImage(imageId);
}, { immediate: true });
watch(() => [props.open, props.serverId] as const, ([open]) => {
  testRunGeneration++;
  if (open) {
    cancelTestRequested = false;
    void initialize();
    void retryPendingImageCleanup();
  } else {
    dialogGeneration++;
    appearanceImageRequest++;
    cancelTestRequested = true;
    void cancelTest();
    secret.value = "";
    key.value = null;
    if (importedImageIds.value.length) {
      const imported = [...importedImageIds.value];
      importedImageIds.value = [];
      void cleanupImageIds(imported);
    }
  }
}, { immediate: true });
watch(() => draft.authType, () => { key.value = null; secret.value = ""; visible.value = false; });
watch(credentialAction, () => { secret.value = ""; visible.value = false; });

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

const discardMessage = computed(() => profileCommittedInSession.value && appearanceDirty.value
  ? t("partialDiscardConfirm")
  : discardAction.value === "reload" ? t("reloadConfirm") : t("discardConfirm"));

function requestClose() {
  if (busy.value) return;
  removeCredentialOpen.value = false;
  if (hasUnsavedChanges.value) {
    discardAction.value = "close";
    discardDialogOpen.value = true;
    return;
  }
  void finishClose();
}

async function finishClose() {
  cancelTestRequested = true;
  testRunGeneration++;
  void cancelTest();
  secret.value = "";
  key.value = null;
  const imported = [...importedImageIds.value];
  importedImageIds.value = [];
  const cleaned = await cleanupImageIds(imported);
  if (!cleaned) emit("saved", t("imageCleanupPending"));
  emit("close");
}

function requestReload() {
  if (busy.value) return;
  if (hasUnsavedChanges.value) {
    discardAction.value = "reload";
    discardDialogOpen.value = true;
    return;
  }
  void initialize();
}

async function confirmDiscard() {
  if (discardBusy.value) return;
  discardBusy.value = true;
  const action = discardAction.value;
  const imported = [...importedImageIds.value];
  importedImageIds.value = [];
  const cleaned = await cleanupImageIds(imported);
  discardDialogOpen.value = false;
  if (!cleaned) emit("saved", t("imageCleanupPending"));
  if (action === "reload") {
    await initialize();
  } else {
    cancelTestRequested = true;
    testRunGeneration++;
    void cancelTest();
    secret.value = "";
    key.value = null;
    emit("close");
  }
  discardBusy.value = false;
}

async function testConnection() {
  if (!props.connectionStore || formBusy.value) return;
  refreshKeyStatus();
  const profile = normalizeServerDraft(draft, key.value);
  const credentialUpdate = resolveCredentialUpdate(
    credentialAction.value,
    secret.value,
    current.value?.hasSavedCredential ?? false,
  );
  const invalid = validateServerDraft(profile, current.value, key.value)
    ?? credentialValidation(current.value, profile, credentialUpdate);
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
      && (current.value.hasPrivateKey || (credentialAction.value === "unchanged" && current.value.hasSavedCredential))
      ? {
          serverId: current.value.id,
          expectedRevision: current.value.revision,
          useSavedCredential: credentialAction.value === "unchanged" && current.value.hasSavedCredential,
        }
      : undefined;
    const draftCredential = !current.value || credentialAction.value === "editing" ? secret.value || null : null;
    const snapshot = await props.connectionStore.startDraftTest(profile, draftCredential, savedProfile);
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

function beginCredentialEdit() {
  credentialAction.value = enterReplace();
  credentialStorageEditing.value = false;
}

function cancelCredentialEdit() {
  credentialAction.value = cancelReplace(credentialAction.value);
  secret.value = "";
  visible.value = false;
}

function openCredentialMenuAction(action: string) {
  if (action === "reveal" && current.value?.hasSavedCredential) emit("revealCredential", current.value.id);
  if (action === "change-storage") credentialStorageEditing.value = true;
  if (action === "remove") removeCredentialOpen.value = true;
}

function confirmCredentialRemoval() {
  credentialAction.value = markPendingClear(credentialAction.value);
  removeCredentialOpen.value = false;
}

function cancelCredentialStorageEdit() {
  draft.requireAuthentication = current.value?.requireAuthentication ?? false;
  credentialStorageEditing.value = false;
}

function keepCredentialStorageChoice() {
  credentialStorageEditing.value = false;
}

async function selectKey() {
  if (formBusy.value) return;
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
  } finally {
    refreshKeyStatus();
    selectingKey.value = false;
  }
}

function acceptCommittedProfile(profile: ServerProfile) {
  current.value = profile;
  savedProfileSnapshot.value = structuredClone(profile);
  Object.assign(draft, newServerDraft(profile));
  credentialAction.value = initialCredentialEditAction(false);
  secret.value = "";
  key.value = null;
}

function buildAppearancePayload(): ServerAppearanceUpdate {
  if (!current.value) throw new Error("Cannot save appearance before the server profile exists");
  return {
    serverId: current.value.id,
    expectedRevision: appearanceRevision.value,
    ...appearanceBusinessValue(),
  };
}

async function saveAll() {
  if (formBusy.value || (props.serverId && !current.value)) return;
  refreshKeyStatus();
  failure.value = null;
  error.value = "";
  appearanceError.value = "";
  const profile = normalizeServerDraft(draft, key.value);
  const credential: CredentialUpdate = resolveCredentialUpdate(
    credentialAction.value,
    secret.value,
    current.value?.hasSavedCredential ?? false,
  );
  const saveProfile = !current.value || profileDirty.value;
  const saveAppearance = !!current.value && appearanceDirty.value;
  const previousAppearanceImageId = originalAppearanceImageId.value;
  const invalid = saveProfile
    ? validateServerDraft(profile, current.value, key.value) ?? credentialValidation(current.value, profile, credential)
    : null;
  if (invalid) {
    section.value = validationSection(invalid);
    errorSection.value = section.value;
    phase.value = "validation";
    error.value = t(invalid);
    return;
  }
  if (saveAppearance && appearanceDraft.terminalAppearance.themeMode === "image" && !appearanceDraft.terminalAppearance.backgroundImage.imageId) {
    section.value = "appearance";
    appearanceError.value = t("imageRequired");
    return;
  }
  if (current.value && !saveProfile && !saveAppearance) {
    await finishClose();
    return;
  }
  phase.value = "saving";
  let credentialCleanupPending = false;
  if (!current.value) {
    try {
      const result = await props.store.create({ profile, credential });
      secret.value = "";
      key.value = null;
      emit("saved", result.credentialCleanupPending ? t("cleanup") : t("added"));
      emit("close");
    } catch (reason) {
      failure.value = mapError(reason);
      errorSection.value = validationSection("hostRequired");
      error.value = presentError(failure.value, props.language).message;
      phase.value = "error";
    }
    return;
  }

  if (saveProfile) {
    try {
      const result = await props.store.update({ serverId: current.value.id, expectedRevision: current.value.revision, profile, credential });
      credentialCleanupPending = result.credentialCleanupPending;
      acceptCommittedProfile(result.server);
      profileCommittedInSession.value = true;
    } catch (reason) {
      failure.value = mapError(reason);
      if (failure.value.code === "REVISION_CONFLICT") section.value = "basic";
      errorSection.value = failure.value.code === "REVISION_CONFLICT" ? "basic" : section.value;
      error.value = presentError(failure.value, props.language).message;
      phase.value = "error";
      return;
    }
  }

  if (saveAppearance) {
    try {
      const saved = await props.appearanceStore.update(buildAppearancePayload());
      loadAppearanceDraft(saved);
      appearanceError.value = "";
    } catch (reason) {
      failure.value = mapError(reason);
      section.value = "appearance";
      errorSection.value = "appearance";
      appearanceError.value = presentError(failure.value, props.language).message;
      phase.value = "error";
      return;
    }
  }

  const cleanupCandidates = [...importedImageIds.value];
  if (previousAppearanceImageId && previousAppearanceImageId !== appearanceDraft.terminalAppearance.backgroundImage.imageId) {
    cleanupCandidates.push(previousAppearanceImageId);
  }
  const cleaned = await cleanupImageIds(cleanupCandidates, appearanceDraft.terminalAppearance.backgroundImage.imageId);
  importedImageIds.value = [];
  const messages = [current.value ? t("updated") : t("added")];
  if (credentialCleanupPending) messages.push(t("cleanup"));
  if (!cleaned) messages.push(t("imageCleanupPending"));
  emit("saved", messages.join(" "));
  emit("close");
}
</script>

<template>
  <BaseDialog :open="open" size="form" :title="t(serverId ? 'edit' : 'add')" :busy="busy" :close-label="t('cancel')" initial-focus="input" panel-class="server-dialog" @close="requestClose">
    <div v-if="phase === 'loading'" class="server-dialog-loading" role="status">{{ t('loading') }}</div>
    <TabsRoot v-else v-model="section" class="server-dialog-layout">
      <TabsList class="server-dialog-tabs" :aria-label="t('serverConfigSections')">
        <TabsTrigger value="basic" class="server-dialog-tab" :disabled="formBusy">{{ t('basicInfo') }}</TabsTrigger>
        <TabsTrigger value="advanced" class="server-dialog-tab" :disabled="formBusy">{{ t('advancedConfig') }}</TabsTrigger>
        <TabsTrigger v-if="serverId" value="appearance" class="server-dialog-tab" :disabled="formBusy">{{ t('appearance') }}</TabsTrigger>
      </TabsList>
      <form :id="formId" class="server-dialog-form" :data-state="phase" novalidate @submit.prevent="saveAll">
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
            <div v-if="draft.authType === 'privateKey'" class="server-field-full base-field server-key-field">
              <div class="server-key-label">
                <label :for="`${formId}-private-key`">{{ t('privateKey') }}</label>
                <BaseInfoPopover :title="t('keyHelp')" :description="t('keyNote')" :trigger-label="t('keyHelp')" side="right" :disabled="formBusy" />
              </div>
              <div class="server-key-control">
                <div class="server-key-picker-input" :data-state="privateKeyUiState">
                  <BaseIcon name="file-text" class="server-key-file-icon" />
                  <div class="server-key-file-details"><span class="server-key-file-name" :title="privateKeyDisplayText">{{ privateKeyDisplayText }}</span></div>
                  <button :id="`${formId}-private-key`" type="button" class="server-key-browse-button" :disabled="formBusy" :aria-label="privateKeyButtonText" @click="selectKey">{{ selectingKey ? t('selectingKey') : privateKeyButtonText }}</button>
                </div>
                <p v-if="privateKeyUiState === 'selected' || privateKeyUiState === 'existing' || privateKeyUiState === 'expired'" class="server-key-file-hint" :class="{ 'server-key-file-hint--error': privateKeyUiState === 'expired' }">
                  {{ privateKeyUiState === 'selected' ? t('selectedKeyHint') : privateKeyUiState === 'existing' ? t('existingKeyHint') : t('expiredKeyHint') }}
                </p>
              </div>
            </div>
            <div v-if="current && credentialAction !== 'editing'" class="server-field-full base-field server-credential-field">
              <div class="server-credential-label">
                <span>{{ t(current.authType === 'privateKey' ? 'passphrase' : 'password') }}</span>
                <BaseInfoPopover v-if="current.authType === 'privateKey'" :title="t('credentialHelp')" :description="t('credentialNote')" :trigger-label="t('credentialHelp')" side="right" :disabled="formBusy" />
              </div>
              <div class="server-credential-control">
                <div class="server-credential-state" :data-state="credentialAction" role="status">
                  <span>{{ credentialAction === 'pendingClear' ? t('credentialRemovalPending') : current.hasSavedCredential ? t(current.authType === 'privateKey' ? 'credentialSavedPassphrase' : 'credentialSavedPassword') : t('credentialNotSaved') }}</span>
                  <BaseButton v-if="credentialAction === 'unchanged'" :disabled="formBusy" @click="beginCredentialEdit">{{ current.hasSavedCredential ? t('credentialChange') : t('credentialAdd') }}</BaseButton>
                  <BaseDropdownMenu v-if="credentialAction === 'unchanged' && current.hasSavedCredential" :label="t('credentialMoreActions')" :items="credentialMenuItems" :disabled="formBusy" @action="openCredentialMenuAction" />
                  <BaseButton v-if="credentialAction === 'pendingClear'" :disabled="formBusy" @click="credentialAction = undoPendingClear(credentialAction)">{{ t('credentialUndoRemoval') }}</BaseButton>
                </div>
                <p v-if="credentialAction === 'unchanged'" class="server-form-note server-credential-storage-summary">{{ credentialStorageSummary }}</p>
                <p v-if="credentialAction === 'pendingClear'" class="server-form-note server-credential-removal-note">{{ t('credentialRemovalDraftNote') }}</p>
                <p v-if="credentialIdentityChanged" class="server-form-note server-credential-identity-note" role="alert">{{ t('identityChanged') }}</p>
              </div>
            </div>
            <div v-if="!current || credentialAction === 'editing'" class="server-field-full server-secret-field">
              <BaseInput v-model="secret" :label="t(draft.authType === 'privateKey' ? 'passphrase' : 'password')" :type="visible ? 'text' : 'password'" autocomplete="new-password" />
              <BaseTooltip :label="visible ? '隐藏凭据 / Hide credential' : '显示凭据 / Show credential'"><button type="button" class="server-secret-toggle" :aria-pressed="visible" :aria-label="visible ? '隐藏凭据 / Hide credential' : '显示凭据 / Show credential'" @click="visible = !visible"><BaseIcon :name="visible ? 'eye-off' : 'eye'" /></button></BaseTooltip>
              <p class="server-form-note">{{ t('secretNote') }}</p>
              <BaseButton v-if="current" class="server-credential-edit-cancel" :disabled="formBusy" @click="cancelCredentialEdit">{{ t(current.hasSavedCredential ? 'credentialCancelReplace' : 'credentialCancelAdd') }}</BaseButton>
            </div>
            <template v-if="!current || credentialAction === 'editing' || credentialStorageEditing">
              <BaseSelect v-model="credentialStorageChoice" class="server-field-full server-credential-storage-field" :label="t('credentialStorage')" :disabled="busy" :options="[{value:'system',label:t('credentialStorageSystem')},{value:'local',label:t('credentialStorageLocal')}]">
                <template #label-suffix><BaseInfoPopover :title="t('credentialStorageHelp')" :description="t('credentialStorageModeNote')" :trigger-label="t('credentialStorageHelp')" side="right" :disabled="formBusy" /></template>
              </BaseSelect>
              <div v-if="current && credentialAction !== 'editing'" class="server-field-full server-credential-storage-actions">
                <BaseButton :disabled="formBusy" @click="cancelCredentialStorageEdit">{{ t('credentialStorageCancel') }}</BaseButton>
                <BaseButton :disabled="formBusy" @click="keepCredentialStorageChoice">{{ t('credentialStorageDone') }}</BaseButton>
              </div>
            </template>
          </fieldset>
          <p v-if="error && errorSection === 'basic'" role="alert" class="server-form-error">{{ error }}</p>
          <BaseButton v-if="(failure?.code === 'REVISION_CONFLICT' && errorSection === 'basic') || (serverId && !current)" :disabled="busy" @click="initialize">{{ t(current ? 'reload' : 'retry') }}</BaseButton>
        </TabsContent>
        <TabsContent value="advanced" class="server-dialog-panel">
          <fieldset :disabled="formBusy || (!!serverId && !current)" class="server-advanced-sections">
            <section class="server-advanced-section">
              <header class="server-advanced-section-heading"><h3>{{ t('organizationClassification') }}</h3></header>
              <BaseSelect v-model="draft.groupId" :label="t('group')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('ungrouped')},...groups.map(group => ({value:group.id,label:group.name}))]" />
              <BaseSelect v-model="appearanceDraft.environment" :label="t('environment')" :disabled="busy || (!!serverId && !current)" :options="[{value:null,label:t('noEnvironment')},{value:'production',label:t('production')},{value:'staging',label:t('staging')},{value:'development',label:t('development')},{value:'custom',label:t('customEnvironment')}]" />
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
          <ServerAppearancePanel
            :model-value="appearancePanelModel"
            :settings="globalSettings"
            :language="language"
            :background-url="appearanceImageUrl"
            :background-name="appearanceImageName"
            :background-busy="imageBusy"
            :background-error="appearanceImageError"
            :global-background-url="globalImageUrl"
            :global-background-busy="globalImageBusy"
            :disabled="formBusy || !current"
            @update:model-value="updateAppearancePanel"
            @select-image="chooseAppearanceImage"
            @remove-image="removeAppearanceImage"
          />
          <BaseAlert v-if="profileCommittedInSession && appearanceDirty" role="alert">{{ t('profileSavedAppearanceFailed') }}</BaseAlert>
          <p v-if="appearanceError" role="alert" class="server-form-error">{{ appearanceError }}</p>
          <p v-if="globalImageError" role="status" class="server-form-error">{{ globalImageError }}</p>
          <BaseButton v-if="failure?.code === 'REVISION_CONFLICT' && errorSection === 'appearance'" type="button" :disabled="busy" @click="requestReload">{{ t('reload') }}</BaseButton>
        </TabsContent>
      </form>
    </TabsRoot>
    <template #footer>
      <div class="server-dialog-footer-leading">
        <BaseButton v-if="connectionStore" :disabled="busy" :loading="testRunning" @click="testConnection">{{ t(testRunning ? 'testingConnection' : 'testConnection') }}</BaseButton>
      </div>
      <div class="server-dialog-footer-actions">
        <BaseButton :disabled="busy" @click="requestClose">{{ t('cancel') }}</BaseButton>
        <BaseButton variant="primary" type="submit" :form="formId" :disabled="saveDisabled" :loading="phase === 'saving'">{{ t(phase === 'saving' ? 'saving' : 'save') }}</BaseButton>
      </div>
    </template>
  </BaseDialog>
  <BaseAlertDialog :open="removeCredentialOpen" :title="t('credentialRemovalTitle')" :close-label="t('cancel')" panel-class="server-credential-removal-dialog" :busy="formBusy" @close="removeCredentialOpen = false">
    <p>{{ t('credentialRemovalConfirm') }}</p>
    <template #footer>
      <BaseButton data-dialog-cancel :disabled="formBusy" @click="removeCredentialOpen = false">{{ t('cancel') }}</BaseButton>
      <BaseButton variant="danger" :disabled="formBusy" @click="confirmCredentialRemoval">{{ t('credentialRemove') }}</BaseButton>
    </template>
  </BaseAlertDialog>
  <BaseAlertDialog :open="discardDialogOpen" :title="t(discardAction === 'reload' ? 'reload' : 'discardTitle')" :close-label="t('keepEditing')" panel-class="server-discard-dialog" :busy="discardBusy" @close="discardDialogOpen = false">
    <p>{{ discardMessage }}</p>
    <template #footer>
      <BaseButton data-dialog-cancel :disabled="discardBusy" @click="discardDialogOpen = false">{{ t('keepEditing') }}</BaseButton>
      <BaseButton variant="danger" :disabled="discardBusy" :loading="discardBusy" @click="confirmDiscard">{{ t(discardAction === 'reload' ? 'reload' : 'discardChanges') }}</BaseButton>
    </template>
  </BaseAlertDialog>
</template>
