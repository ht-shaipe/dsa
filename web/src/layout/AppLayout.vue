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
import { useUpdater } from '@/composables/useUpdater'
import { ElNotification } from 'element-plus'

const appStore = useAppStore()

onMounted(() => {
  appStore.initTheme()

  const autoCheck = localStorage.getItem('dsa_auto_update_check')
  if (autoCheck !== 'false' && typeof window !== 'undefined' && ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)) {
    setTimeout(async () => {
      try {
        const updater = useUpdater()
        const hasUpdate = await updater.checkForUpdate(true)
        if (hasUpdate && updater.updateInfo.value) {
          ElNotification({
            title: '发现新版本',
            message: `v${updater.updateInfo.value.version} 已发布，前往「系统设置 → 更新管理」查看详情`,
            type: 'info',
            duration: 8000,
          })
        }
      } catch {
        // silent fail
      }
    }, 5000)
  }
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
  padding: 0 0 6px 0;
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
