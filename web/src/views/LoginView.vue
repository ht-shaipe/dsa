<template>
  <div class="login-page">
    <div class="login-card">
      <div class="login-logo">
        <img src="@/assets/icon.png" class="logo-img" />
        <span class="logo-text">DSA</span>
      </div>
      <p class="login-subtitle">AI 驱动的每日股票分析系统</p>

      <el-form @submit.prevent="handleSubmit">
        <el-form-item>
          <el-input
            v-model="mobile"
            placeholder="请输入手机号"
            size="large"
            maxlength="11"
            clearable
          >
            <template #prefix>
              <el-icon><Iphone /></el-icon>
            </template>
          </el-input>
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="password"
            type="password"
            placeholder="请输入密码"
            show-password
            size="large"
            @keyup.enter="handleSubmit"
          >
            <template #prefix>
              <el-icon><Lock /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <template v-if="mode === 'register'">
          <el-form-item>
            <el-input
              v-model="regPassword2"
              type="password"
              placeholder="确认密码"
              show-password
              size="large"
            >
              <template #prefix>
                <el-icon><Lock /></el-icon>
              </template>
            </el-input>
          </el-form-item>
          <el-form-item>
            <el-input
              v-model="regName"
              placeholder="昵称（选填）"
              size="large"
              clearable
            >
              <template #prefix>
                <el-icon><User /></el-icon>
              </template>
            </el-input>
          </el-form-item>
          <el-form-item>
            <div class="sms-row">
              <el-input
                v-model="smsCode"
                placeholder="验证码"
                size="large"
                maxlength="6"
                clearable
              >
                <template #prefix>
                  <el-icon><Message /></el-icon>
                </template>
              </el-input>
              <el-button
                size="large"
                :disabled="smsCooldown > 0 || !mobile"
                :loading="sendingCode"
                @click="sendSmsCode"
                style="width: 130px; flex-shrink: 0"
              >
                {{ smsCooldown > 0 ? `${smsCooldown}s` : '获取验证码' }}
              </el-button>
            </div>
          </el-form-item>
        </template>

        <el-form-item>
          <el-button
            type="primary"
            size="large"
            :loading="loading"
            style="width: 100%"
            @click="handleSubmit"
          >
            {{ mode === 'register' ? '注 册' : '登 录' }}
          </el-button>
        </el-form-item>

        <div class="mode-switch">
          <template v-if="mode === 'login'">
            还没有账号？<el-link type="primary" @click="mode = 'register'">立即注册</el-link>
          </template>
          <template v-else>
            已有账号？<el-link type="primary" @click="mode = 'login'">返回登录</el-link>
          </template>
        </div>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import { getApiBase } from '@/api/index'
import axios from 'axios'

const authStore = useAuthStore()
const router = useRouter()

const mobile = ref('')
const password = ref('')
const regName = ref('')
const regPassword2 = ref('')
const smsCode = ref('')
const mode = ref<'login' | 'register'>('login')
const loading = ref(false)
const sendingCode = ref(false)
const smsCooldown = ref(0)
let cooldownTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  if (authStore.isAuthenticated) {
    router.replace('/')
  }
})

async function sendSmsCode() {
  if (!mobile.value || !/^1\d{10}$/.test(mobile.value)) {
    ElMessage.warning('请输入正确的手机号')
    return
  }

  sendingCode.value = true
  try {
    const { data } = await axios.post(`${getApiBase()}/auth/mobile-exists`, { mobile: mobile.value })
    if (data?.code === 200 && data?.result === true) {
      ElMessage.warning('该手机号已注册，请直接登录')
      mode.value = 'login'
      return
    }
  } catch { /* ignore, proceed to send code */ }

  try {
    const { data } = await axios.post(`${getApiBase()}/auth/sms-code`, { mobile: mobile.value })
    if (data?.code === 200) {
      ElMessage.success('验证码已发送')
      smsCooldown.value = 60
      cooldownTimer = setInterval(() => {
        smsCooldown.value--
        if (smsCooldown.value <= 0 && cooldownTimer) {
          clearInterval(cooldownTimer)
          cooldownTimer = null
        }
      }, 1000)
    } else {
      ElMessage.error(data?.message || '发送验证码失败')
    }
  } catch (err: any) {
    ElMessage.error(err?.response?.data?.message || '发送验证码失败')
  } finally {
    sendingCode.value = false
  }
}

async function handleSubmit() {
  if (loading.value) return

  if (!mobile.value) {
    ElMessage.warning('请输入手机号')
    return
  }
  if (!/^1\d{10}$/.test(mobile.value)) {
    ElMessage.warning('请输入正确的手机号')
    return
  }
  if (!password.value) {
    ElMessage.warning('请输入密码')
    return
  }
  if (password.value.length < 6) {
    ElMessage.warning('密码至少6位')
    return
  }

  if (mode.value === 'register') {
    if (password.value !== regPassword2.value) {
      ElMessage.warning('两次密码不一致')
      return
    }
    if (!smsCode.value) {
      ElMessage.warning('请输入验证码')
      return
    }
    loading.value = true
    try {
      const { data } = await axios.post(`${getApiBase()}/auth/register`, {
        mobile: mobile.value,
        password: password.value,
        code: smsCode.value,
        name: regName.value || undefined,
      })
      if (data?.code === 200) {
        ElMessage.success('注册成功，请登录')
        mode.value = 'login'
        smsCode.value = ''
      } else {
        ElMessage.error(data?.message || '注册失败')
      }
    } catch (err: any) {
      ElMessage.error(err?.response?.data?.message || '注册失败')
    } finally {
      loading.value = false
    }
    return
  }

  loading.value = true
  try {
    const err = await authStore.loginWithRemote(mobile.value, password.value)
    if (!err) {
      ElMessage.success('登录成功')
      router.replace('/')
    } else {
      ElMessage.error(err)
    }
  } catch {
    // error already shown
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
.login-logo {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 8px;
}
.logo-img {
  width: 40px;
  height: 40px;
  object-fit: contain;
}
.logo-text {
  font-size: 28px;
  font-weight: bold;
  color: var(--el-color-primary);
}
.login-subtitle {
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin-bottom: 30px;
}
.sms-row {
  display: flex;
  gap: 8px;
  width: 100%;
}
.mode-switch {
  text-align: center;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}
</style>
