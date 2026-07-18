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
      <BottomStatusBar />
    </el-container>
    
  </el-container>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import SidebarNav from './SidebarNav.vue'
import AppHeader from './AppHeader.vue'
import BottomStatusBar from '@/components/common/BottomStatusBar.vue'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'
import { useUpdater } from '@/composables/useUpdater'
import { useAutoSync } from '@/composables/useAutoSync'
import { ElNotification } from 'element-plus'

const appStore = useAppStore()
const taskStore = useTaskStore()
const { startAutoCheck, stopAutoCheck } = useAutoSync()

onMounted(() => {
  appStore.initTheme()
  taskStore.connect()
  taskStore.refreshAllStatus()
  startAutoCheck(60)

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

onUnmounted(() => {
  taskStore.disconnect()
  stopAutoCheck()
})
</script>

<style scoped lang="scss">
.app-layout {
  height: 100vh;
  overflow: hidden;
}
.app-aside {
  background: var(--dsa-sidebar-bg);
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
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.08), 0 1px 2px -1px rgba(0, 0, 0, 0.06);
}
.app-main {
  background: var(--dsa-bg);
  padding: 0 0 6px 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.app-scroll {
  flex: 1;
  padding: 16px;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
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
