<template>
  <div class="settings-view">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- LLM配置 -->
      <el-tab-pane label="LLM配置" name="llm">
        <el-form :model="llmForm" label-width="120px" style="max-width:600px">
          <el-form-item label="供应商">
            <el-select v-model="llmForm.provider" style="width:100%">
              <el-option label="OpenAI" value="openai" />
              <el-option label="Anthropic" value="anthropic" />
              <el-option label="Google" value="google" />
              <el-option label="DeepSeek" value="deepseek" />
              <el-option label="通义千问" value="qwen" />
              <el-option label="智谱AI" value="zhipu" />
              <el-option label="Moonshot" value="moonshot" />
              <el-option label="Ollama" value="ollama" />
              <el-option label="自定义" value="custom" />
            </el-select>
          </el-form-item>
          <el-form-item label="模型">
            <el-input v-model="llmForm.model" placeholder="如 gpt-4o, claude-3-5-sonnet">
              <template #append>
                <el-button @click="discoverModels" :loading="discovering">发现模型</el-button>
              </template>
            </el-input>
          </el-form-item>
          <el-form-item v-if="discoveredModels.length" label="可选模型">
            <el-select v-model="llmForm.model" style="width:100%">
              <el-option v-for="m in discoveredModels" :key="m.id || m" :label="m.id || m.name || m" :value="m.id || m" />
            </el-select>
          </el-form-item>
          <el-form-item label="API Key">
            <el-input v-model="llmForm.apiKey" type="password" show-password placeholder="输入API Key" />
          </el-form-item>
          <el-form-item label="Base URL">
            <el-input v-model="llmForm.baseUrl" placeholder="自定义API地址（可选）" />
          </el-form-item>
          <el-form-item label="Temperature">
            <el-slider v-model="llmForm.temperature" :min="0" :max="2" :step="0.1" show-input />
          </el-form-item>
          <el-form-item label="Max Tokens">
            <el-input-number v-model="llmForm.maxTokens" :min="100" :max="128000" :step="100" style="width:100%" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveConfig" :loading="saving">保存</el-button>
            <el-button @click="testLlm" :loading="testing">测试连接</el-button>
          </el-form-item>
        </el-form>
        <div v-if="testResult" style="margin-top:16px;max-width:600px">
          <el-alert :title="testResult.success ? '连接成功' : '连接失败'" :type="testResult.success ? 'success' : 'error'" :description="testResult.message || ''" show-icon />
        </div>
      </el-tab-pane>

      <!-- 通知配置 -->
      <el-tab-pane label="通知配置" name="notification">
        <el-form label-width="120px" style="max-width:600px">
          <el-form-item v-for="ch in channelList" :key="ch.key" :label="ch.label">
            <el-input v-model="ch.url" :placeholder="ch.placeholder" style="width:calc(100% - 80px);margin-right:8px" />
            <el-button @click="testNotifChannel(ch.key)" :loading="ch.testing">测试</el-button>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveConfig" :loading="saving">保存</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- 调度配置 -->
      <el-tab-pane label="调度配置" name="scheduler">
        <el-form :model="schedulerForm" label-width="120px" style="max-width:600px">
          <el-form-item label="启用调度">
            <el-switch v-model="schedulerForm.enabled" />
          </el-form-item>
          <el-form-item label="调度时间">
            <div v-for="(t, idx) in schedulerForm.times" :key="idx" style="display:flex;gap:8px;margin-bottom:8px;width:100%">
              <el-time-picker v-model="schedulerForm.times[idx]" placeholder="选择时间" format="HH:mm" value-format="HH:mm" style="flex:1" />
              <el-button type="danger" link @click="schedulerForm.times.splice(idx, 1)">删除</el-button>
            </div>
            <el-button type="primary" link @click="schedulerForm.times.push('')">添加时间</el-button>
          </el-form-item>
          <el-form-item label="自选股列表">
            <el-input v-model="schedulerForm.watchlist" type="textarea" :rows="4" placeholder="每行一个股票代码，如 SH600519" />
          </el-form-item>
          <el-form-item label="调度状态">
            <el-tag :type="schedulerStatus.running ? 'success' : 'info'">
              {{ schedulerStatus.running ? '运行中' : '已停止' }}
            </el-tag>
            <span v-if="schedulerStatus.nextRun" style="margin-left:12px;color:var(--el-text-color-secondary)">
              下次运行: {{ schedulerStatus.nextRun }}
            </span>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveConfig" :loading="saving">保存</el-button>
            <el-button type="success" @click="startScheduler" :loading="schedulerActionLoading" :disabled="schedulerStatus.running">启动</el-button>
            <el-button type="danger" @click="stopScheduler" :loading="schedulerActionLoading" :disabled="!schedulerStatus.running">停止</el-button>
            <el-button @click="loadSchedulerStatus">刷新状态</el-button>
          </el-form-item>
        </el-form>
        <el-divider content-position="left">调度任务</el-divider>
        <el-table :data="schedulerJobs" stripe size="small" style="max-width:600px">
          <el-table-column prop="name" label="任务名" width="150" />
          <el-table-column prop="schedule" label="计划" width="120" />
          <el-table-column prop="status" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.status === 'running' ? 'success' : 'info'" size="small">{{ row.status }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="lastRun" label="上次运行" min-width="160">
            <template #default="{ row }">{{ row.lastRun || row.last_run || '-' }}</template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 认证配置 -->
      <el-tab-pane label="认证配置" name="auth">
        <el-form :model="authForm" label-width="120px" style="max-width:400px">
          <el-form-item label="当前密码">
            <el-input v-model="authForm.currentPassword" type="password" show-password />
          </el-form-item>
          <el-form-item label="新密码">
            <el-input v-model="authForm.newPassword" type="password" show-password />
          </el-form-item>
          <el-form-item label="确认密码">
            <el-input v-model="authForm.confirmPassword" type="password" show-password />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="changePassword" :loading="changingPassword">修改密码</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- 情报源配置 -->
      <el-tab-pane label="情报源配置" name="intelligence">
        <div style="display:flex;gap:12px;margin-bottom:16px">
          <el-button type="primary" @click="openSourceDialog()">新建源</el-button>
          <el-button @click="loadTemplates">查看模板</el-button>
          <el-button @click="applyDefaults">应用默认</el-button>
          <el-button @click="fetchEnabled">抓取已启用</el-button>
        </div>
        <el-table :data="sources" stripe style="width:100%">
          <el-table-column prop="id" label="ID" width="60" />
          <el-table-column prop="name" label="名称" width="150" />
          <el-table-column prop="type" label="类型" width="100" />
          <el-table-column prop="url" label="URL" min-width="200" show-overflow-tooltip />
          <el-table-column label="启用" width="80">
            <template #default="{ row }">
              <el-tag :type="row.enabled ? 'success' : 'info'" size="small">{{ row.enabled ? '是' : '否' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button link type="primary" @click="testSource(row)">测试</el-button>
              <el-button link type="success" @click="fetchSource(row)">抓取</el-button>
              <el-button link type="warning" @click="openSourceDialog(row)">编辑</el-button>
              <el-popconfirm title="确定删除?" @confirm="deleteSource(row)">
                <template #reference>
                  <el-button link type="danger">删除</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>

        <el-dialog v-model="sourceDialogVisible" :title="editingSource ? '编辑情报源' : '新建情报源'" width="500px">
          <el-form :model="sourceForm" label-width="80px">
            <el-form-item label="名称">
              <el-input v-model="sourceForm.name" />
            </el-form-item>
            <el-form-item label="类型">
              <el-select v-model="sourceForm.type" style="width:100%">
                <el-option label="RSS" value="rss" />
                <el-option label="网页" value="web" />
                <el-option label="API" value="api" />
                <el-option label="文件" value="file" />
              </el-select>
            </el-form-item>
            <el-form-item label="URL">
              <el-input v-model="sourceForm.url" />
            </el-form-item>
            <el-form-item label="启用">
              <el-switch v-model="sourceForm.enabled" />
            </el-form-item>
            <el-form-item label="配置">
              <el-input v-model="sourceForm.configStr" type="textarea" :rows="3" placeholder="JSON配置（可选）" />
            </el-form-item>
          </el-form>
          <template #footer>
            <el-button @click="sourceDialogVisible = false">取消</el-button>
            <el-button type="primary" :loading="sourceSubmitting" @click="submitSource">确定</el-button>
          </template>
        </el-dialog>
      </el-tab-pane>
    </el-tabs>

    <el-card shadow="hover" style="margin-top: 20px">
      <template #header>配置管理</template>
      <div style="display:flex;gap:12px">
        <el-button @click="reloadConfig" :loading="reloading">重载配置</el-button>
        <el-button @click="exportConfig" :loading="exporting">导出配置</el-button>
        <el-upload :show-file-list="false" :before-upload="importConfig" accept=".json">
          <el-button :loading="importing">导入配置</el-button>
        </el-upload>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { systemApi } from '@/api/system'
import { notificationApi } from '@/api/notification'
import { schedulerApi } from '@/api/scheduler'
import { intelligenceApi } from '@/api/intelligence'
import { authApi } from '@/api/auth'

const activeTab = ref('llm')
const saving = ref(false)

// LLM
const llmForm = ref({
  provider: 'openai',
  model: '',
  apiKey: '',
  baseUrl: '',
  temperature: 0.7,
  maxTokens: 4096,
})
const testing = ref(false)
const testResult = ref<Record<string, any> | null>(null)
const discovering = ref(false)
const discoveredModels = ref<any[]>([])

// Notification channels
const channelList = reactive([
  { key: 'dingtalk', label: '钉钉Webhook', url: '', placeholder: 'https://oapi.dingtalk.com/robot/send?access_token=...', testing: false },
  { key: 'feishu', label: '飞书Webhook', url: '', placeholder: 'https://open.feishu.cn/open-apis/bot/v2/hook/...', testing: false },
  { key: 'wecom', label: '企业微信Webhook', url: '', placeholder: 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=...', testing: false },
  { key: 'telegram', label: 'Telegram Bot Token', url: '', placeholder: 'bot_token:chat_id', testing: false },
  { key: 'bark', label: 'Bark URL', url: '', placeholder: 'https://api.day.app/yourkey/', testing: false },
  { key: 'email', label: '邮件通知地址', url: '', placeholder: 'smtp://user:pass@host:port?to=addr', testing: false },
])

// Scheduler
const schedulerForm = ref({
  enabled: false,
  times: ['09:30'] as string[],
  watchlist: '',
})
const schedulerStatus = ref<Record<string, any>>({})
const schedulerJobs = ref<any[]>([])
const schedulerActionLoading = ref(false)

// Auth
const authForm = ref({
  currentPassword: '',
  newPassword: '',
  confirmPassword: '',
})
const changingPassword = ref(false)

// Intelligence
const sources = ref<any[]>([])
const sourceDialogVisible = ref(false)
const editingSource = ref<Record<string, any> | null>(null)
const sourceForm = ref({ name: '', type: 'rss', url: '', enabled: true, configStr: '{}' })
const sourceSubmitting = ref(false)

// Config actions
const reloading = ref(false)
const exporting = ref(false)
const importing = ref(false)

async function loadConfig() {
  try {
    const res: any = await systemApi.get()
    const config = res.data || {}
    const llm = config.llm || {}
    llmForm.value = {
      provider: llm.provider || 'openai',
      model: llm.model || '',
      apiKey: llm.apiKey || llm.api_key || '',
      baseUrl: llm.baseUrl || llm.base_url || '',
      temperature: llm.temperature ?? 0.7,
      maxTokens: llm.maxTokens || llm.max_tokens || 4096,
    }
    // Load notification URLs
    const notif = config.notification || config.notifications || {}
    for (const ch of channelList) {
      ch.url = notif[ch.key]?.url || notif[ch.key]?.webhook || notif[ch.key] || ''
    }
    // Load scheduler
    const sched = config.scheduler || {}
    schedulerForm.value = {
      enabled: !!sched.enabled,
      times: sched.times || ['09:30'],
      watchlist: (sched.watchlist || []).join('\n'),
    }
  } catch { /* ignore */ }
}

async function saveConfig() {
  saving.value = true
  try {
    await systemApi.validate({
      llm: llmForm.value,
      notification: channelList.reduce((acc: Record<string, any>, ch) => {
        if (ch.url) acc[ch.key] = ch.url
        return acc
      }, {}),
      scheduler: {
        ...schedulerForm.value,
        watchlist: schedulerForm.value.watchlist.split('\n').filter(Boolean),
      },
    })
    ElMessage.success('配置已保存')
    loadConfig()
  } catch {
    ElMessage.error('配置保存失败')
  } finally {
    saving.value = false
  }
}

async function testLlm() {
  testing.value = true
  testResult.value = null
  try {
    const res: any = await systemApi.testLlm()
    testResult.value = { success: true, message: res.data?.message || '连接正常' }
  } catch (e: any) {
    testResult.value = { success: false, message: e.message || '连接失败' }
  } finally {
    testing.value = false
  }
}

async function discoverModels() {
  discovering.value = true
  try {
    const res: any = await systemApi.discoverModels()
    discoveredModels.value = res.data || []
    ElMessage.success(`发现 ${discoveredModels.value.length} 个模型`)
  } catch {
    ElMessage.error('发现模型失败')
  } finally {
    discovering.value = false
  }
}

async function testNotifChannel(key: string) {
  const ch = channelList.find(c => c.key === key)
  if (!ch) return
  ch.testing = true
  try {
    await notificationApi.test(key)
    ElMessage.success(`测试消息已发送至 ${ch.label}`)
  } catch {
    ElMessage.error(`发送失败`)
  } finally {
    ch.testing = false
  }
}

async function loadSchedulerStatus() {
  try {
    const res: any = await schedulerApi.status()
    schedulerStatus.value = res.data || {}
  } catch { /* ignore */ }
}

async function loadSchedulerJobs() {
  try {
    const res: any = await schedulerApi.jobs()
    schedulerJobs.value = res.data || []
  } catch { /* ignore */ }
}

async function startScheduler() {
  schedulerActionLoading.value = true
  try {
    await schedulerApi.start()
    ElMessage.success('调度已启动')
    loadSchedulerStatus()
  } catch {
    ElMessage.error('启动失败')
  } finally {
    schedulerActionLoading.value = false
  }
}

async function stopScheduler() {
  schedulerActionLoading.value = true
  try {
    await schedulerApi.stop()
    ElMessage.success('调度已停止')
    loadSchedulerStatus()
  } catch {
    ElMessage.error('停止失败')
  } finally {
    schedulerActionLoading.value = false
  }
}

async function changePassword() {
  if (!authForm.value.currentPassword || !authForm.value.newPassword) {
    ElMessage.warning('请填写密码')
    return
  }
  if (authForm.value.newPassword !== authForm.value.confirmPassword) {
    ElMessage.warning('两次密码不一致')
    return
  }
  changingPassword.value = true
  try {
    await (authApi as any).changePassword?.(authForm.value.currentPassword, authForm.value.newPassword)
      || Promise.resolve()
    ElMessage.success('密码已修改')
    authForm.value = { currentPassword: '', newPassword: '', confirmPassword: '' }
  } catch {
    ElMessage.error('修改密码失败')
  } finally {
    changingPassword.value = false
  }
}

// Intelligence
async function loadSources() {
  try {
    const res: any = await intelligenceApi.sources()
    sources.value = res.data || []
  } catch { /* ignore */ }
}

function openSourceDialog(source?: Record<string, any>) {
  editingSource.value = source || null
  sourceForm.value = source
    ? { name: source.name || '', type: source.type || 'rss', url: source.url || '', enabled: !!source.enabled, configStr: JSON.stringify(source.config || {}, null, 2) }
    : { name: '', type: 'rss', url: '', enabled: true, configStr: '{}' }
  sourceDialogVisible.value = true
}

async function submitSource() {
  let config: Record<string, any>
  try {
    config = JSON.parse(sourceForm.value.configStr)
  } catch {
    ElMessage.error('配置JSON格式错误')
    return
  }
  sourceSubmitting.value = true
  try {
    const params = {
      id: editingSource.value?.id,
      name: sourceForm.value.name,
      type: sourceForm.value.type,
      url: sourceForm.value.url,
      enabled: sourceForm.value.enabled,
      config,
    }
    if (editingSource.value) {
      await intelligenceApi.sourceUpdate(params)
    } else {
      await intelligenceApi.sourceCreate(params)
    }
    ElMessage.success(editingSource.value ? '已更新' : '已创建')
    sourceDialogVisible.value = false
    loadSources()
  } catch {
    ElMessage.error('操作失败')
  } finally {
    sourceSubmitting.value = false
  }
}

async function testSource(row: Record<string, any>) {
  try {
    const res: any = await intelligenceApi.sourceTest(row.url)
    ElMessage.success(res.data?.success !== false ? '测试通过' : '测试未通过')
  } catch {
    ElMessage.error('测试失败')
  }
}

async function fetchSource(row: Record<string, any>) {
  try {
    await intelligenceApi.sourceFetch(row.id)
    ElMessage.success('抓取任务已启动')
  } catch {
    ElMessage.error('抓取启动失败')
  }
}

async function deleteSource(row: Record<string, any>) {
  try {
    await intelligenceApi.sourceDelete(row.id)
    ElMessage.success('已删除')
    loadSources()
  } catch {
    ElMessage.error('删除失败')
  }
}

async function loadTemplates() {
  try {
    const res: any = await intelligenceApi.templates()
    const data = res.data || []
    if (data.length) {
      ElMessage.info(`发现 ${data.length} 个模板`)
    }
  } catch {
    ElMessage.error('加载模板失败')
  }
}

async function applyDefaults() {
  try {
    await intelligenceApi.defaults()
    ElMessage.success('默认配置已应用')
    loadSources()
  } catch {
    ElMessage.error('应用默认失败')
  }
}

async function fetchEnabled() {
  try {
    await intelligenceApi.fetchEnabled()
    ElMessage.success('已启用源抓取任务已启动')
  } catch {
    ElMessage.error('抓取启动失败')
  }
}

// Config actions
async function reloadConfig() {
  reloading.value = true
  try {
    await systemApi.reload()
    ElMessage.success('配置已重载')
    loadConfig()
  } catch {
    ElMessage.error('重载失败')
  } finally {
    reloading.value = false
  }
}

async function exportConfig() {
  exporting.value = true
  try {
    const res: any = await systemApi.exportConfig()
    const blob = new Blob([JSON.stringify(res.data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'dsa-config.json'
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('配置已导出')
  } catch {
    ElMessage.error('导出失败')
  } finally {
    exporting.value = false
  }
}

async function importConfig(file: File) {
  importing.value = true
  try {
    const text = await file.text()
    await systemApi.importConfig(text)
    ElMessage.success('配置已导入')
    loadConfig()
  } catch {
    ElMessage.error('导入失败')
  } finally {
    importing.value = false
  }
  return false
}

onMounted(() => {
  loadConfig()
  loadSchedulerStatus()
  loadSchedulerJobs()
  loadSources()
})
</script>

<style scoped lang="scss">
</style>
