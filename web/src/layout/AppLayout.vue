<template>
  <el-container class="app-layout">
    <el-aside :width="appStore.sidebarCollapsed ? '64px' : '220px'" class="app-aside">
      <SidebarNav />
    </el-aside>
    <el-container>
      <el-header class="app-header">
        <AppHeader />
      </el-header>
      <el-main class="app-main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import SidebarNav from './SidebarNav.vue'
import AppHeader from './AppHeader.vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

onMounted(() => {
  appStore.initTheme()
})
</script>

<style scoped lang="scss">
.app-layout {
  height: 100vh;
}
.app-aside {
  background: var(--el-menu-bg-color);
  border-right: 1px solid var(--el-border-color-light);
  transition: width 0.3s;
  overflow: hidden;
}
.app-header {
  border-bottom: 1px solid var(--el-border-color-light);
  display: flex;
  align-items: center;
  padding: 0 20px;
  height: 56px;
}
.app-main {
  background: var(--dsa-bg);
  padding: 20px;
  overflow-y: auto;
}
</style>
