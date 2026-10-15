<script setup lang="ts">
import type { ConnectionSnapshot } from "../../../../contracts/v1/ConnectionSnapshot";
import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";
import type { MonitorStore } from "../../stores/monitor";
import type { TerminalTab } from "../../terminal/controller";
import { messages } from "../../i18n/locale";
import { terminalMessages } from "../../i18n/terminal";
import { monitorMessages } from "../../i18n/monitor";
import { connectionStateText } from "../../i18n/connections";
import BaseButton from "../base/BaseButton.vue";
import BaseIcon from "../base/BaseIcon.vue";
import BaseIconButton from "../base/BaseIconButton.vue";
import QuickMonitorPanel from "./QuickMonitorPanel.vue";

const t = messages(terminalMessages);
const monitorText = messages(monitorMessages);
const props = defineProps<{
  server: ServerProfile;
  snapshot: ConnectionSnapshot;
  store: MonitorStore;
  ready: boolean;
  pane: "terminal" | "files" | "monitor";
  terminals: TerminalTab[];
  activeTerminal: TerminalTab | null;
}>();
const emit = defineEmits<{ close: []; full: [] }>();
</script>

<template>
  <aside class="workspace-inspector" :aria-label="t('hostInspector')">
    <header>
      <h2>{{ t('hostInspector') }}</h2>
      <BaseIconButton :label="t('closeInspector')" @click="emit('close')"><BaseIcon name="x" /></BaseIconButton>
    </header>

    <section class="workspace-inspector-section workspace-inspector-host">
      <p class="workspace-inspector-kicker">{{ t('currentHost') }}</p>
      <strong :title="server.name">{{ server.name }}</strong>
      <span class="workspace-inspector-address" :title="`${server.username}@${server.host}`">{{ server.username }}@{{ server.host }}</span>
      <dl>
        <dt>{{ t('connectionStatus') }}</dt><dd>{{ connectionStateText(snapshot.state) }}</dd>
        <dt>{{ t('connectionMethod') }}</dt><dd>SSH / {{ server.port }}</dd>
      </dl>
    </section>

    <QuickMonitorPanel :store="store" :connection-id="snapshot.connectionId" :ready="ready" :show-full-monitor-button="false" />

    <section class="workspace-inspector-section workspace-inspector-session">
      <h3>{{ t('session') }}</h3>
      <dl>
        <dt>{{ t('terminalTabs') }}</dt><dd>{{ t(terminals.length === 1 ? 'oneTerminal' : 'terminalCount', { count: terminals.length }) }}</dd>
        <dt>{{ t('currentTerminal') }}</dt><dd :title="activeTerminal?.title ?? '—'">{{ activeTerminal?.title ?? '—' }}</dd>
        <dt>{{ t('currentView') }}</dt><dd>{{ t(pane) }}</dd>
      </dl>
      <BaseButton v-if="pane !== 'monitor'" variant="secondary" size="sm" :disabled="!ready" @click="emit('full')">{{ monitorText('openFullMonitor') }}</BaseButton>
    </section>
  </aside>
</template>
