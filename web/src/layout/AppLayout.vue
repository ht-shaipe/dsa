<template>
  <el-container class="app-layout">
    <el-aside :width="appStore.sidebarCollapsed ? '72px' : '180px'" class="app-aside" :style="{ width: appStore.sidebarCollapsed ? '72px' : '180px' }">
      <SidebarNav />
    </el-aside>
    <el-container>
      <el-header class="app-header" data-tauri-drag-region>
        <AppHeader />
      </el-header>
      <el-main class="app-main">
        <el-scrollbar class="app-scroll">
          <router-view />
        </el-scrollbar>
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
  overflow: hidden;
}
.app-aside {
  background: var(--dsa-bg);
  border-right: 1px solid var(--el-border-color-light);
  transition: width 0.3s;
  overflow: visible;
  padding-top: 0;
  width: auto !important;
  min-width: auto !important;
}
.app-header {
  border-bottom: 1px solid var(--el-border-color-light);
  display: flex;
  align-items: center;
  padding: 0 20px;
  height: 56px;
  user-select: none;
  position: relative;
  z-index: 10;
  -webkit-app-region: drag;
}
.app-main {
  background: var(--dsa-bg);
  padding: 0px;
  overflow: hidden;
}

.app-scroll {
    padding: 16px;
}
</style>

<style>
/* 覆盖 Element Plus el-aside 的默认宽度 */
.app-layout > .el-aside.el-aside {
  width: auto !important;
  min-width: 72px !important;
  max-width: none !important;
}
</style>
