import { createApp, h, type Component } from "vue";
import App from "./App.vue";
import BaseUiProvider from "./components/base/BaseUiProvider.vue";
const createUiApp = (component: Component) => createApp({ render: () => h(BaseUiProvider, null, () => h(component)) });
import "./styles/tokens.css";
import "./styles/themes.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/overlays.css";
import "./styles/shell.css";
import "./styles/servers.css";
import "./styles/connections.css";
import "./styles/server-overview.css";
import "@xterm/xterm/css/xterm.css";
import "./styles/terminal.css";
import "./styles/files.css";
import "./styles/monitor.css";
import "./styles/settings.css";

if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "interaction") {
  void import("./harness/InteractionHarness.vue").then(({ default: InteractionHarness }) => { createUiApp(InteractionHarness).mount("#app"); });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "visual") {
  void import("./harness/VisualHarness.vue").then(({ default: VisualHarness }) => { createUiApp(VisualHarness).mount("#app"); });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "settings") {
  void import("./harness/SettingsHarness.vue").then(({ default: SettingsHarness }) => { createUiApp(SettingsHarness).mount("#app"); });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "monitor") {
  void import("./harness/MonitorHarness.vue").then(({ default: MonitorHarness }) => { createUiApp(MonitorHarness).mount("#app"); });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "files") {
  void import("./harness/FilesHarness.vue").then(({ default: FilesHarness }) => { createUiApp(FilesHarness).mount("#app"); });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "terminal") {
  void import("./harness/TerminalHarness.vue").then(({ default: TerminalHarness }) => {
    createUiApp(TerminalHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "connections") {
  void import("./harness/ConnectionHarness.vue").then(({ default: ConnectionHarness }) => {
    createUiApp(ConnectionHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "servers") {
  void import("./harness/ServerHarness.vue").then(({ default: ServerHarness }) => {
    createUiApp(ServerHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "foundations") {
  void import("./harness/FoundationsHarness.vue").then(({ default: FoundationsHarness }) => {
    createUiApp(FoundationsHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "shell") {
  void import("./harness/ShellHarness.vue").then(({ default: ShellHarness }) => {
    createUiApp(ShellHarness).mount("#app");
  });
} else {
  createUiApp(App).mount("#app");
}
