<template>
  <div class="login-page">
    <div class="login-card">
      <h2 class="login-title">DSA 登录</h2>
      <el-form @submit.prevent="handleLogin">
        <el-form-item>
          <el-input
            v-model="password"
            type="password"
            placeholder="请输入密码"
            show-password
            size="large"
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" size="large" :loading="loading" style="width: 100%" @click="handleLogin">
            登 录
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'

const authStore = useAuthStore()
const router = useRouter()
const password = ref('')
const loading = ref(false)

onMounted(async () => {
  await authStore.checkStatus()
  if (authStore.isAuthenticated) {
    router.replace('/')
  }
})

async function handleLogin() {
  if (!password.value && authStore.authEnabled) {
    ElMessage.warning('请输入密码')
    return
  }
  loading.value = true
  try {
    const ok = await authStore.login(password.value)
    if (ok) {
      ElMessage.success('登录成功')
      router.replace('/')
    } else {
      ElMessage.error('密码错误')
    }
  } finally {
    loading.value = false
  }
}
</script>

<style scoped lang="scss">
.login-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--dsa-bg);
}
.login-card {
  width: 400px;
  padding: 40px;
  border-radius: 12px;
  background: var(--el-bg-color);
  box-shadow: var(--el-box-shadow-light);
}
.login-title {
  text-align: center;
  margin-bottom: 30px;
  color: var(--el-color-primary);
}
</style>
