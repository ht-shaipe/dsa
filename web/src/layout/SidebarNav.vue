<template>
  <div class="sidebar-wrapper">
    <el-scrollbar class="sidebar-scrollbar">
    <el-menu
      :default-active="route.path"
      :collapse="appStore.sidebarCollapsed"
      router
      class="sidebar-menu"
    >
      <div class="logo-area">
        <img src="@/assets/icon.png" class="logo-icon" />
        <span v-if="!appStore.sidebarCollapsed" class="logo-text">DSA</span>
      </div>

      <el-menu-item index="/">
        <el-icon><DataAnalysis /></el-icon>
        <template #title>工作台</template>
      </el-menu-item>
      <el-menu-item index="/watchlist">
        <el-icon><Star /></el-icon>
        <template #title>自选股</template>
      </el-menu-item>
      <el-menu-item index="/chat">
        <el-icon><ChatDotRound /></el-icon>
        <template #title>AI 问股</template>
      </el-menu-item>
      <el-menu-item index="/analysis-history">
        <el-icon><Document /></el-icon>
        <template #title>分析历史</template>
      </el-menu-item>
      <el-menu-item index="/screening">
        <el-icon><Filter /></el-icon>
        <template #title>选股筛选</template>
      </el-menu-item>
      <el-menu-item index="/portfolio">
        <el-icon><Wallet /></el-icon>
        <template #title>投资组合</template>
      </el-menu-item>
      <el-menu-item index="/decision-signals">
        <el-icon><Guide /></el-icon>
        <template #title>决策信号</template>
      </el-menu-item>
      <el-menu-item index="/backtest">
        <el-icon><TrendCharts /></el-icon>
        <template #title>回测分析</template>
      </el-menu-item>
      <el-menu-item index="/alerts">
        <el-icon><Bell /></el-icon>
        <template #title>预警中心</template>
      </el-menu-item>
      <el-menu-item index="/stock-pool">
        <el-icon><List /></el-icon>
        <template #title>股票池</template>
      </el-menu-item>
      <el-menu-item index="/usage">
        <el-icon><Odometer /></el-icon>
        <template #title>用量统计</template>
      </el-menu-item>
      <el-menu-item index="/settings">
        <el-icon><Setting /></el-icon>
        <template #title>系统设置</template>
      </el-menu-item>

    </el-menu>
  </el-scrollbar>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const appStore = useAppStore()
</script>

<style scoped lang="scss">
.sidebar-wrapper {
  height: 100%;
  position: relative;
  overflow: hidden;
}

.sidebar-scrollbar {
  height: 100%;
  overflow-x: hidden;
  overflow-y: auto;
  :deep(.el-scrollbar__wrap) {
    overflow-x: hidden;
  }
  :deep(.el-scrollbar__view) {
    width: 100%;
  }
}

.sidebar-menu {
  border-right: none;
  background: transparent;
  --el-menu-item-bg-color: transparent;
  --el-menu-item-hover-bg-color: var(--el-fill-color-light);
  --el-menu-item-active-bg-color: var(--el-color-primary-light-9);
  padding: 0;
  width: 100%;
  :deep(.el-menu-item) {
    width: 100%;
  }
}
.logo-area {
  height: 35px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  // border-bottom: 1px solid var(--el-border-color-light);
  padding-top: 30px;
  width: 100%;
  padding-left: 0;
  padding-right: 0;
  margin-left: 0;
}
.logo-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}
.logo-text {
  font-size: 20px;
  font-weight: bold;
  color: var(--el-color-primary);
}
</style>
