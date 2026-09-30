<script setup lang="ts">
import AppShell from "./app/AppShell.vue";
import { createIpcClient } from "./ipc/client";
const client = createIpcClient();
</script>

<template><AppShell :client="client" /></template>
