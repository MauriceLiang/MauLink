import { createApp } from "vue";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/themes.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/shell.css";
import "./styles/servers.css";
import "./styles/connections.css";

if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "connections") {
  void import("./harness/ConnectionHarness.vue").then(({ default: ConnectionHarness }) => {
    createApp(ConnectionHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "servers") {
  void import("./harness/ServerHarness.vue").then(({ default: ServerHarness }) => {
    createApp(ServerHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "foundations") {
  void import("./harness/FoundationsHarness.vue").then(({ default: FoundationsHarness }) => {
    createApp(FoundationsHarness).mount("#app");
  });
} else if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "shell") {
  void import("./harness/ShellHarness.vue").then(({ default: ShellHarness }) => {
    createApp(ShellHarness).mount("#app");
  });
} else {
  createApp(App).mount("#app");
}
