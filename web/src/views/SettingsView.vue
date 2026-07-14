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
          <el-form-item label="自选股">
            <router-link to="/watchlist" style="color:var(--el-color-primary)">前往自选股管理</router-link>
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

      <!-- 数据同步 -->
      <el-tab-pane label="数据同步" name="data_sync">
        <el-form :model="dataSyncForm" label-width="130px" style="max-width:650px">
          <el-divider content-position="left">同步范围</el-divider>
          <el-form-item label="市场板块">
            <el-checkbox-group v-model="dataSyncForm.boards">
              <el-checkbox label="沪市主板" value="sh_main" />
              <el-checkbox label="深市主板" value="sz_main" />
              <el-checkbox label="创业板" value="sz_gem" />
              <el-checkbox label="科创板" value="sh_kj" />
              <el-checkbox label="北交所" value="bj_main" />
            </el-checkbox-group>
          </el-form-item>
          <el-divider content-position="left">风险过滤</el-divider>
          <el-form-item label="排除ST股票">
            <el-switch v-model="dataSyncForm.excludeSt" />
            <span style="margin-left:8px;color:var(--el-text-color-secondary)">过滤名称含ST/*ST的股票</span>
          </el-form-item>
          <el-form-item label="排除退市风险">
            <el-switch v-model="dataSyncForm.excludeDelistingRisk" />
            <span style="margin-left:8px;color:var(--el-text-color-secondary)">过滤名称含退市/退的股票</span>
          </el-form-item>
          <el-form-item label="排除次新股">
            <el-switch v-model="dataSyncForm.excludeNewStock" />
            <span style="margin-left:8px;color:var(--el-text-color-secondary)">上市不足60天的股票波动大、数据少</span>
          </el-form-item>
          <el-divider content-position="left">数据管理</el-divider>
          <el-form-item label="保留天数">
            <el-input-number v-model="dataSyncForm.retentionDays" :min="60" :max="1000" :step="30" style="width:180px" />
            <span style="margin-left:8px;color:var(--el-text-color-secondary)">超过此天数的日线数据将被清理</span>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveDataSyncConfig" :loading="saving">保存配置</el-button>
          </el-form-item>
        </el-form>

        <el-divider content-position="left">操作</el-divider>
        <div style="display:flex;gap:12px;align-items:center;margin-bottom:16px">
          <el-button type="primary" @click="initDailyData" :loading="syncRunning" :disabled="syncRunning">
            {{ syncRunning ? '同步进行中...' : '初始化日线数据' }}
          </el-button>
          <el-button type="danger" @click="cleanDailyData" :loading="cleaning" :disabled="syncRunning">清理过期数据</el-button>
          <el-button @click="loadSyncStatus">刷新状态</el-button>
        </div>

        <el-card v-if="syncStatus.running || syncStatus.total > 0" shadow="never" style="max-width:650px">
          <div style="display:flex;align-items:center;gap:12px;margin-bottom:8px">
            <el-tag :type="syncStatus.running ? 'warning' : 'success'">
              {{ syncStatus.running ? '同步中' : (syncStatus.phase === 'done' ? '已完成' : '未开始') }}
            </el-tag>
            <span v-if="syncStatus.phase && syncStatus.phase !== 'done'" style="color:var(--el-text-color-secondary)">{{ syncStatus.phase }}</span>
          </div>
          <el-progress
            v-if="syncStatus.total > 0"
            :percentage="Math.round((syncStatus.done / syncStatus.total) * 100)"
            :status="syncStatus.running ? '' : 'success'"
            :format="() => `${syncStatus.done} / ${syncStatus.total}`"
          />
          <div v-if="syncStatus.failed > 0" style="margin-top:4px;color:var(--el-color-danger)">
            失败: {{ syncStatus.failed }}
          </div>
        </el-card>
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

      <!-- 更新管理 -->
      <el-tab-pane label="更新管理" name="update">
        <template v-if="!updater.isTauri">
          <el-alert title="自动更新仅在桌面客户端中可用" description="请下载安装 DSA 桌面版以使用自动更新功能" type="info" show-icon :closable="false" />
        </template>
        <template v-else>
          <el-form label-width="120px" style="max-width:600px">
            <el-form-item label="当前版本">
              <span style="font-weight:600">{{ appVersion }}</span>
            </el-form-item>
            <el-form-item label="更新状态">
              <el-tag :type="statusTagType">{{ statusLabel }}</el-tag>
              <span v-if="updater.status.value === 'available' && updater.updateInfo.value" style="margin-left:12px;color:var(--el-text-color-secondary)">
                新版本 {{ updater.updateInfo.value.version }} 可用
              </span>
            </el-form-item>
            <el-form-item v-if="updater.updateInfo.value" label="新版本信息">
              <div>
                <div style="font-weight:600;margin-bottom:6px">v{{ updater.updateInfo.value.version }}
                  <span v-if="updater.updateInfo.value.date" style="font-size:12px;color:var(--el-text-color-secondary);margin-left:8px">
                    {{ updater.updateInfo.value.date }}
                  </span>
                </div>
                <div v-if="updater.updateInfo.value.body" style="white-space:pre-wrap;font-size:13px;color:var(--el-text-color-regular);line-height:1.6">
                  {{ updater.updateInfo.value.body }}
                </div>
              </div>
            </el-form-item>
            <el-form-item v-if="updater.status.value === 'downloading'" label="下载进度">
              <el-progress
                :percentage="downloadPercent"
                :format="() => `${downloadedMB} / ${totalMB}`"
                :stroke-width="18"
                style="width:100%"
              />
            </el-form-item>
            <el-form-item v-if="updater.errorMessage.value" label="">
              <el-alert :title="updater.errorMessage.value" type="error" show-icon :closable="false" />
            </el-form-item>
            <el-form-item>
              <el-button
                type="primary"
                :loading="updater.status.value === 'checking'"
                :disabled="updater.status.value === 'checking' || updater.status.value === 'downloading'"
                @click="updater.checkForUpdate()"
              >
                {{ updater.status.value === 'checking' ? '检查中...' : '检查更新' }}
              </el-button>
              <el-button
                v-if="updater.status.value === 'available' || updater.status.value === 'downloading'"
                type="success"
                :loading="updater.status.value === 'downloading'"
                @click="updater.downloadAndInstall()"
              >
                {{ updater.status.value === 'downloading' ? '下载中...' : '下载并安装' }}
              </el-button>
              <el-button
                v-if="updater.status.value === 'ready'"
                type="success"
                @click="updater.restartApp()"
              >
                立即重启应用更新
              </el-button>
            </el-form-item>
          </el-form>

          <el-divider content-position="left">自动更新</el-divider>
          <el-form label-width="120px" style="max-width:600px">
            <el-form-item label="启动时检查">
              <el-switch v-model="autoCheckEnabled" @change="(val: any) => saveAutoCheck(!!val)" />
              <span style="margin-left:8px;color:var(--el-text-color-secondary)">每次启动应用时自动检查新版本</span>
            </el-form-item>
          </el-form>
        </template>
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
import { ref, onMounted, reactive, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { systemApi } from '@/api/system'
import { notificationApi } from '@/api/notification'
import { schedulerApi } from '@/api/scheduler'
import { intelligenceApi } from '@/api/intelligence'
import { useUpdater } from '@/composables/useUpdater'

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
})
const schedulerStatus = ref<Record<string, any>>({})
const schedulerJobs = ref<any[]>([])
const schedulerActionLoading = ref(false)

// Data sync
const dataSyncForm = ref({
  boards: ['sh_main', 'sz_main', 'sz_gem'] as string[],
  excludeSt: true,
  excludeNewStock: true,
  excludeDelistingRisk: true,
  retentionDays: 120,
})
const syncStatus = ref<Record<string, any>>({})
const syncRunning = ref(false)
const cleaning = ref(false)
let syncPollTimer: ReturnType<typeof setInterval> | null = null

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

const updater = useUpdater()
const appVersion = ref('0.1.0')
const autoCheckEnabled = ref(localStorage.getItem('dsa_auto_update_check') !== 'false')

const statusTagType = computed(() => {
  switch (updater.status.value) {
    case 'idle': return 'info'
    case 'checking': return 'warning'
    case 'available': return 'success'
    case 'downloading': return 'warning'
    case 'ready': return 'success'
    case 'error': return 'danger'
    default: return 'info'
  }
})

const statusLabel = computed(() => {
  switch (updater.status.value) {
    case 'idle': return '已是最新版本'
    case 'checking': return '检查中...'
    case 'available': return '发现新版本'
    case 'downloading': return '下载中'
    case 'ready': return '更新就绪'
    case 'error': return '更新出错'
    case 'unsupported': return '不支持自动更新'
    default: return '未知'
  }
})

const downloadPercent = computed(() => {
  const { downloaded, total } = updater.progress.value
  if (!total || total === 0) return 0
  return Math.min(Math.round((downloaded / total) * 100), 100)
})

const downloadedMB = computed(() => (updater.progress.value.downloaded / 1048576).toFixed(1) + ' MB')
const totalMB = computed(() => {
  const t = updater.progress.value.total
  return t > 0 ? (t / 1048576).toFixed(1) + ' MB' : '未知'
})

function saveAutoCheck(val: boolean) {
  localStorage.setItem('dsa_auto_update_check', val ? 'true' : 'false')
}

async function loadConfig() {
  try {
    const res: any = await systemApi.get()
    const config = res || {}
    const llm = config.llm || {}
    llmForm.value = {
      provider: llm.provider || 'openai',
      model: llm.model || '',
      apiKey: llm.api_key || llm.apiKey || llm.api_key_env || '',
      baseUrl: llm.base_url || llm.baseUrl || '',
      temperature: llm.temperature ?? 0.7,
      maxTokens: llm.max_tokens || llm.maxTokens || llm.timeout_seconds || 4096,
    }
    // Load notification URLs
    const notif = config.notification || config.notifications || {}
    for (const ch of channelList) {
      ch.url = notif[`${ch.key}_webhook`] || notif[ch.key]?.url || notif[ch.key]?.webhook || notif[ch.key] || ''
      if (ch.key === 'telegram') {
        const token = notif.telegram_bot_token || ''
        const chatId = notif.telegram_chat_id || ''
        ch.url = token && chatId ? `${token}:${chatId}` : token || ''
      }
      if (ch.key === 'email') {
        const host = notif.email_smtp_host || ''
        const port = notif.email_smtp_port || 465
        const user = notif.email_user || ''
        const pass = notif.email_pass || ''
        const to = notif.email_to || ''
        ch.url = host ? `smtp://${user}:${pass}@${host}:${port}?to=${to}` : ''
      }
    }
    // Load scheduler
    const sched = config.scheduler || {}
    schedulerForm.value = {
      enabled: !!sched.enabled,
      times: sched.times || ['09:30'],
    }
    // Load data sync
    const ds = config.data_sync || config.dataSync || {}
    dataSyncForm.value = {
      boards: ds.boards || ['sh_main', 'sz_main', 'sz_gem'],
      excludeSt: ds.excludeSt ?? ds.exclude_st ?? true,
      excludeNewStock: ds.excludeNewStock ?? ds.exclude_new_stock ?? true,
      excludeDelistingRisk: ds.excludeDelistingRisk ?? ds.exclude_delisting_risk ?? true,
      retentionDays: ds.retentionDays ?? ds.retention_days ?? 120,
    }
  } catch { /* ignore */ }
}

async function saveConfig() {
  saving.value = true
  try {
    await systemApi.save({
      llm: {
        provider: llmForm.value.provider,
        api_key: llmForm.value.apiKey,
        base_url: llmForm.value.baseUrl,
        model: llmForm.value.model,
        temperature: llmForm.value.temperature,
        max_tokens: llmForm.value.maxTokens,
      },
      notification: (() => {
        const n: Record<string, any> = {}
        for (const ch of channelList) {
          if (!ch.url) continue
          if (ch.key === 'telegram') {
            const parts = ch.url.split(':')
            n.telegram_bot_token = parts[0] || ''
            n.telegram_chat_id = parts.slice(1).join(':') || ''
          } else if (ch.key === 'email') {
            try {
              const u = new URL(ch.url)
              n.email_smtp_host = u.hostname
              n.email_smtp_port = parseInt(u.port) || 465
              n.email_user = decodeURIComponent(u.username)
              n.email_pass = decodeURIComponent(u.password)
              n.email_to = u.searchParams.get('to') || ''
              n.email_from = n.email_user
            } catch { n.email_smtp_host = ch.url }
          } else {
            n[`${ch.key}_webhook`] = ch.url
          }
        }
        return n
      })(),
      scheduler: schedulerForm.value,
      data_sync: {
        boards: dataSyncForm.value.boards,
        exclude_st: dataSyncForm.value.excludeSt,
        exclude_new_stock: dataSyncForm.value.excludeNewStock,
        exclude_delisting_risk: dataSyncForm.value.excludeDelistingRisk,
        retention_days: dataSyncForm.value.retentionDays,
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
    testResult.value = { success: true, message: res?.message || '连接正常' }
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
    discoveredModels.value = Array.isArray(res) ? res : []
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
    schedulerStatus.value = res || {}
  } catch { /* ignore */ }
}

async function loadSchedulerJobs() {
  try {
    const res: any = await schedulerApi.jobs()
    schedulerJobs.value = Array.isArray(res) ? res : []
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

async function loadSyncStatus() {
  try {
    const res: any = await systemApi.syncStatus()
    syncStatus.value = res || {}
    syncRunning.value = !!res?.running
    if (res?.config) {
      dataSyncForm.value = {
        boards: res.config.boards || ['sh_main', 'sz_main', 'sz_gem'],
        excludeSt: res.config.excludeSt ?? true,
        excludeNewStock: res.config.excludeNewStock ?? true,
        excludeDelistingRisk: res.config.excludeDelistingRisk ?? true,
        retentionDays: res.config.retentionDays ?? 120,
      }
    }
  } catch { /* ignore */ }
}

async function initDailyData() {
  try {
    const res: any = await systemApi.initDailyData()
    ElMessage.success(res?.message || '同步已启动')
    syncRunning.value = true
    startSyncPolling()
  } catch {
    // error already shown by api interceptor
  }
}

async function cleanDailyData() {
  cleaning.value = true
  try {
    const res: any = await systemApi.cleanDailyData()
    ElMessage.success(`已清理 ${res?.deleted ?? 0} 条过期数据`)
  } catch {
    // error already shown by api interceptor
  } finally {
    cleaning.value = false
  }
}

async function saveDataSyncConfig() {
  saving.value = true
  try {
    await systemApi.save({
      data_sync: dataSyncForm.value,
    })
    ElMessage.success('数据同步配置已保存')
  } catch {
    // error already shown by api interceptor
  } finally {
    saving.value = false
  }
}

function startSyncPolling() {
  if (syncPollTimer) clearInterval(syncPollTimer)
  syncPollTimer = setInterval(() => {
    loadSyncStatus()
    if (!syncRunning.value && syncPollTimer) {
      clearInterval(syncPollTimer)
      syncPollTimer = null
    }
  }, 3000)
}

// Intelligence
async function loadSources() {
  try {
    const res: any = await intelligenceApi.sources()
    sources.value = Array.isArray(res) ? res : []
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
    ElMessage.success(res?.success !== false ? '测试通过' : '测试未通过')
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
    const data = res || []
    if (Array.isArray(data) && data.length) {
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
    const blob = new Blob([JSON.stringify(res, null, 2)], { type: 'application/json' })
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

onMounted(async () => {
  loadConfig()
  loadSchedulerStatus()
  loadSchedulerJobs()
  loadSources()
  loadSyncStatus()
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    appVersion.value = await getVersion()
  } catch {
    appVersion.value = '0.1.0'
  }
})
</script>

<style scoped lang="scss">
</style>
