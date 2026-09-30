<script setup lang="ts">
import BaseButton from "../components/base/BaseButton.vue";
import ShellIcon from "./ShellIcon.vue";
defineProps<{ hasServers: boolean }>();
defineEmits<{ about: [] }>();
</script>

<template>
  <section class="shell-welcome" aria-labelledby="shell-welcome-title">
    <div class="shell-welcome-icon" aria-hidden="true"><ShellIcon name="server" /></div>
    <h1 id="shell-welcome-title">{{ hasServers ? '选择一台服务器' : '还没有服务器' }}</h1>
    <p>{{ hasServers ? '从左侧选择服务器，查看服务器信息。' : '连接你的第一台服务器。\n在一个工作区中使用终端、管理文件并查看服务器状态。' }}</p>
    <BaseButton v-if="!hasServers" variant="primary" disabled title="添加服务器暂不可用"><ShellIcon name="plus" />添加服务器</BaseButton>
    <button class="shell-learn-link" type="button" @click="$emit('about')">了解 MauLink</button>
  </section>
</template>
