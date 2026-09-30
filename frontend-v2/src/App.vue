<template>
  <div class="app-shell">
    <header class="app-header" data-tauri-drag-region>
      <span class="app-brand">MauLink</span>
      <span class="app-version">Frontend v2</span>
    </header>
    <div class="app-content">
      <aside class="app-sidebar" aria-label="服务器工作区">
        <h2 class="sidebar-heading">服务器</h2>
        <p class="sidebar-placeholder">服务器列表将在后续阶段迁移。</p>
      </aside>
      <main class="app-main">
        <section class="shell-placeholder" aria-labelledby="shell-title">
          <span class="brand-mark" aria-hidden="true">M</span>
          <h1 id="shell-title">前端工程已就绪</h1>
          <p>Vue 3 · TypeScript · Vite</p>
        </section>
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100%;
  flex-direction: column;
}

.app-header {
  display: flex;
  min-height: var(--app-toolbar-height);
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-5) 0 88px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}

.app-brand {
  font-size: 16px;
  font-weight: 650;
}

.app-version {
  color: var(--muted);
  font-size: 12px;
}

.app-content {
  display: flex;
  min-height: 0;
  flex: 1;
}

.app-sidebar {
  width: var(--app-sidebar-width);
  flex: none;
  padding: var(--space-5);
  border-right: 1px solid var(--line);
  background: var(--surface-subtle);
}

.sidebar-heading {
  margin: 0 0 var(--space-3);
  font-size: 13px;
  font-weight: 600;
}

.sidebar-placeholder {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.6;
}

.app-main {
  display: grid;
  min-width: 0;
  flex: 1;
  overflow: auto;
  place-items: center;
  padding: var(--space-5);
}

.shell-placeholder {
  text-align: center;
}

.brand-mark {
  display: inline-grid;
  width: 56px;
  height: 56px;
  place-items: center;
  border-radius: var(--radius);
  color: var(--accent);
  background: var(--accent-soft);
  font-size: 28px;
  font-weight: 700;
}

.shell-placeholder h1 {
  margin: var(--space-5) 0 var(--space-3);
  font-size: 24px;
  font-weight: 600;
}

.shell-placeholder p {
  margin: 0;
  color: var(--muted);
}
</style>
