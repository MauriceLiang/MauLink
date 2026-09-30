import { createApp } from "vue";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/themes.css";
import "./styles/base.css";
import "./styles/components.css";

if (import.meta.env.DEV && new URLSearchParams(location.search).get("harness") === "foundations") {
  void import("./harness/FoundationsHarness.vue").then(({ default: FoundationsHarness }) => {
    createApp(FoundationsHarness).mount("#app");
  });
} else {
  createApp(App).mount("#app");
}
