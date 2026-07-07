<template>
  <div class="app-header-inner">
    <el-icon class="collapse-btn" @click="appStore.toggleSidebar">
      <Fold v-if="!appStore.sidebarCollapsed" />
      <Expand v-else />
    </el-icon>
    <div class="header-right">
      <el-switch
        v-model="isDark"
        active-text="暗色"
        inactive-text="亮色"
        @change="() => {}"
        style="--el-switch-on-color: #2c2c2c"
      />
      <el-dropdown @command="handleCommand">
        <span class="user-dropdown">
          <el-icon><User /></el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="logout">退出登录</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'

const appStore = useAppStore()
const authStore = useAuthStore()
const router = useRouter()

const isDark = computed({
  get: () => appStore.isDark,
  set: () => appStore.toggleTheme(),
})

function handleCommand(cmd: string) {
  if (cmd === 'logout') {
    authStore.logout()
    router.push('/login')
  }
}
</script>

<style scoped lang="scss">
.app-header-inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
.collapse-btn {
  cursor: pointer;
  font-size: 20px;
  color: var(--el-text-color-regular);
  &:hover {
    color: var(--el-color-primary);
  }
}
.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}
.user-dropdown {
  cursor: pointer;
  display: flex;
  align-items: center;
}
</style>
