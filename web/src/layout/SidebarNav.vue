<template>
  <el-scrollbar class="sidebar-scrollbar">
    <el-menu
      :default-active="route.path"
      :collapse="appStore.sidebarCollapsed"
      router
      class="sidebar-menu"
    >
      <div class="logo-area">
        <span v-if="!appStore.sidebarCollapsed" class="logo-text">DSA</span>
        <span v-else class="logo-text-mini">D</span>
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
        <template #title>Agent 问股</template>
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
      <el-menu-item index="/usage">
        <el-icon><Odometer /></el-icon>
        <template #title>用量统计</template>
      </el-menu-item>
      <el-menu-item index="/settings">
        <el-icon><Setting /></el-icon>
        <template #title>系统设置</template>
      </el-menu-item>
      <el-menu-item index="/guide">
        <el-icon><Document /></el-icon>
        <template #title>使用手册</template>
      </el-menu-item>
    </el-menu>
  </el-scrollbar>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const appStore = useAppStore()
</script>

<style scoped lang="scss">
.sidebar-scrollbar {
  height: 100%;
}
.sidebar-menu {
  border-right: none;
}
.logo-area {
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-bottom: 1px solid var(--el-border-color-light);
}
.logo-text {
  font-size: 20px;
  font-weight: bold;
  color: var(--el-color-primary);
}
.logo-text-mini {
  font-size: 18px;
  font-weight: bold;
  color: var(--el-color-primary);
}
</style>
