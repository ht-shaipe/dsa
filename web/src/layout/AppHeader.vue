<template>
  <div class="app-header-inner" data-tauri-drag-region>
    <div class="header-left" data-tauri-drag-region>
      <!-- <div class="traffic-light-space"></div> -->
      <el-icon class="collapse-btn" @click="appStore.toggleSidebar">
        <Fold v-if="!appStore.sidebarCollapsed" />
        <Expand v-else />
      </el-icon>
    </div>
    <div class="header-center" data-tauri-drag-region>
      <span class="app-title">DSA - Daily Stock Analysis</span>
    </div>
    <div class="header-right">
      <el-dropdown @command="handleCommand" trigger="click">
        <span class="user-dropdown">
          <el-avatar :size="28" :icon="User" :src="authStore.avatarUrl" />
          <span class="user-name">{{ authStore.displayName }}</span>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <div class="user-info-card">
              <el-avatar :size="48" :icon="User" :src="authStore.avatarUrl" />
              <div class="user-info-text">
                <div class="user-info-name">{{ authStore.userInfo?.name || '未设置昵称' }}</div>
                <div class="user-info-mobile">{{ authStore.userInfo?.mobile || '' }}</div>
              </div>
            </div>
            <el-dropdown-item divided command="profile">
              <el-icon><EditPen /></el-icon>修改个人信息
            </el-dropdown-item>
            <el-dropdown-item command="password">
              <el-icon><Lock /></el-icon>修改密码
            </el-dropdown-item>
            <el-dropdown-item divided command="logout">
              <el-icon><SwitchButton /></el-icon>退出登录
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <el-dropdown trigger="click" @command="handleThemeChange">
        <el-icon class="theme-btn" :title="themeTooltip">
          <Sunny v-if="appStore.themeMode === 'light'" />
          <Moon v-else-if="appStore.themeMode === 'dark'" />
          <Monitor v-else />
        </el-icon>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="system" :class="{ 'is-active': appStore.themeMode === 'system' }">
              <el-icon><Monitor /></el-icon>
              <span>跟随系统</span>
            </el-dropdown-item>
            <el-dropdown-item command="light" :class="{ 'is-active': appStore.themeMode === 'light' }">
              <el-icon><Sunny /></el-icon>
              <span>亮色模式</span>
            </el-dropdown-item>
            <el-dropdown-item command="dark" :class="{ 'is-active': appStore.themeMode === 'dark' }">
              <el-icon><Moon /></el-icon>
              <span>暗色模式</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <el-tooltip content="使用帮助" placement="bottom">
        <el-icon class="help-btn" @click="helpDrawer = true"><QuestionFilled /></el-icon>
      </el-tooltip>
    </div>

    <el-drawer v-model="helpDrawer" title="使用帮助" size="520px" :append-to-body="true">
      <div class="help-content">
        <el-collapse v-model="helpActive">
          <el-collapse-item title="快速开始" name="quick">
            <el-steps direction="vertical" :active="3" :space="50" finish-status="success">
              <el-step title="配置 AI 模型" description="进入「系统设置 → LLM配置」，选择供应商并填入 API Key，点击「测试连接」确认可用" />
              <el-step title="初始化行情数据" description="进入「系统设置 → 数据同步」，选择板块后点击「初始化日线数据」" />
              <el-step title="开始分析" description="在工作台搜索股票，点击「开始分析」即可生成 AI 报告" />
            </el-steps>
          </el-collapse-item>

          <el-collapse-item title="工作台" name="dashboard">
            <ul>
              <li>顶部展示大盘指数实时行情</li>
              <li>搜索框输入股票代码或名称（如 600519、贵州茅台）</li>
              <li>点击「开始分析」，系统调用 AI 生成完整分析报告</li>
              <li>报告包含：情绪评分、操作建议、目标价、风险提示</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="自选股" name="watchlist">
            <ul>
              <li>搜索添加关注的股票到自选列表</li>
              <li>实时显示行情数据、涨跌幅</li>
              <li>可对自选股批量执行 AI 分析</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="Agent 问股" name="chat">
            <ul>
              <li>与 AI Agent 多轮对话，支持追问和深入讨论</li>
              <li>可选择不同策略：技术分析、决策建议等</li>
              <li>Agent 可调用行情查询、技术指标计算等工具</li>
              <li>支持流式输出，实时显示思考过程</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="选股筛选" name="screening">
            <ul>
              <li>选择筛选策略，点击「执行筛选」</li>
              <li>结果含评分和筛选理由</li>
              <li>下方市场热点卡片可查看热点详情和相关股票</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="投资组合" name="portfolio">
            <ul>
              <li>点击「买入/卖出」录入交易</li>
              <li>自动计算持仓成本、浮动盈亏、总收益率</li>
              <li>支持 FIFO 分批成本追踪</li>
              <li>交易记录完整保留</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="决策信号" name="signals">
            <ul>
              <li>系统从 AI 分析中自动提取买卖信号</li>
              <li>可按股票、操作类型、状态筛选</li>
              <li>点击卡片查看入场价、止损价、目标价等详情</li>
              <li>可「采纳」或「拒绝」待处理信号</li>
              <li>提交反馈帮助系统持续优化</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="回测分析" name="backtest">
            <ul>
              <li>输入分析 ID，执行历史回测</li>
              <li>查看胜率、总收益、最大回撤、交易明细</li>
              <li>帮助判断信号在历史数据中的可靠性</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="预警中心" name="alerts">
            <ul>
              <li>新建规则：选择类型（价格突破/涨跌幅/成交量等）</li>
              <li>填写条件，如价格突破 <code>{"field":"price","op":">","value":1800}</code></li>
              <li>启用后触发时自动推送通知</li>
            </ul>
          </el-collapse-item>

          <el-collapse-item title="系统设置" name="settings">
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="LLM 配置">选择供应商、模型、填入 API Key，点击「测试连接」</el-descriptions-item>
              <el-descriptions-item label="数据同步">选择板块、配置风险过滤（ST/退市/次新），初始化日线数据</el-descriptions-item>
              <el-descriptions-item label="通知配置">配置钉钉/飞书/企微/Telegram 等推送渠道</el-descriptions-item>
              <el-descriptions-item label="调度配置">设置定时自动分析（建议收盘后 18:00）</el-descriptions-item>
              <el-descriptions-item label="情报源">管理 RSS/API 新闻源</el-descriptions-item>
            </el-descriptions>
          </el-collapse-item>

          <el-collapse-item title="常见问题" name="faq">
            <el-collapse>
              <el-collapse-item title="AI 分析报错？" name="faq1">
                <ul>
                  <li>确认 API Key 已设置并测试连接成功</li>
                  <li>确认网络能访问 LLM API</li>
                  <li>如需代理，在 config.toml 的 [proxy] 中配置</li>
                </ul>
              </el-collapse-item>
              <el-collapse-item title="行情数据为空？" name="faq2">
                <ul>
                  <li>需联网，数据来自东方财富公开接口</li>
                  <li>非交易时段部分数据可能为空</li>
                  <li>在「系统设置 → 数据同步」初始化日线数据</li>
                </ul>
              </el-collapse-item>
              <el-collapse-item title="日线数据初始化失败？" name="faq3">
                <ul>
                  <li>确认网络可访问东方财富 API（可能需要代理）</li>
                  <li>在 config.toml 的 [proxy] 中配置 http_proxy</li>
                  <li>可在「数据同步」页面查看同步进度</li>
                </ul>
              </el-collapse-item>
            </el-collapse>
          </el-collapse-item>
        </el-collapse>
      </div>
    </el-drawer>

    <el-dialog v-model="profileDialogVisible" title="修改个人信息" width="420px" :append-to-body="true">
      <el-form :model="profileForm" label-width="80px">
        <el-form-item label="昵称">
          <el-input v-model="profileForm.name" placeholder="请输入昵称" maxlength="20" />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input :model-value="authStore.userInfo?.mobile || ''" disabled />
        </el-form-item>
        <el-form-item label="头像URL">
          <el-input v-model="profileForm.avatar" placeholder="头像图片URL（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="profileDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="profileSaving" @click="saveProfile">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="passwordDialogVisible" title="修改密码" width="420px" :append-to-body="true">
      <el-form :model="passwordForm" label-width="90px">
        <el-form-item label="当前密码">
          <el-input v-model="passwordForm.oldPassword" type="password" show-password placeholder="请输入当前密码" />
        </el-form-item>
        <el-form-item label="新密码">
          <el-input v-model="passwordForm.newPassword" type="password" show-password placeholder="请输入新密码（至少6位）" />
        </el-form-item>
        <el-form-item label="确认新密码">
          <el-input v-model="passwordForm.confirmPassword" type="password" show-password placeholder="请再次输入新密码" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="passwordDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="passwordSaving" @click="savePassword">确认修改</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { remoteChangePassword } from '@/api/index'
import { ElMessage } from 'element-plus'
import { QuestionFilled, Sunny, Moon, Monitor, User, EditPen, Lock, SwitchButton } from '@element-plus/icons-vue'

const appStore = useAppStore()
const authStore = useAuthStore()
const router = useRouter()

const themeTooltip = computed(() => {
  switch (appStore.themeMode) {
    case 'system':
      return '跟随系统'
    case 'light':
      return '亮色模式'
    case 'dark':
      return '暗色模式'
    default:
      return '主题'
  }
})

const helpDrawer = ref(false)
const helpActive = ref(['quick'])

const profileDialogVisible = ref(false)
const profileSaving = ref(false)
const profileForm = reactive({ name: '', avatar: '' })

const passwordDialogVisible = ref(false)
const passwordSaving = ref(false)
const passwordForm = reactive({ oldPassword: '', newPassword: '', confirmPassword: '' })

function handleCommand(cmd: string) {
  if (cmd === 'logout') {
    authStore.logout()
    router.push('/login')
  } else if (cmd === 'profile') {
    profileForm.name = authStore.userInfo?.name || ''
    profileForm.avatar = authStore.userInfo?.avatar || ''
    profileDialogVisible.value = true
  } else if (cmd === 'password') {
    passwordForm.oldPassword = ''
    passwordForm.newPassword = ''
    passwordForm.confirmPassword = ''
    passwordDialogVisible.value = true
  }
}

function handleThemeChange(mode: 'system' | 'light' | 'dark') {
  appStore.setTheme(mode)
}

async function saveProfile() {
  if (!profileForm.name.trim()) {
    ElMessage.warning('请输入昵称')
    return
  }
  profileSaving.value = true
  try {
    const ok = await authStore.updateProfile(profileForm.name.trim(), profileForm.avatar.trim() || undefined)
    if (ok) {
      ElMessage.success('个人信息已更新')
      profileDialogVisible.value = false
    } else {
      ElMessage.error('更新失败')
    }
  } catch {
    ElMessage.error('更新失败')
  } finally {
    profileSaving.value = false
  }
}

async function savePassword() {
  if (!passwordForm.oldPassword) {
    ElMessage.warning('请输入当前密码')
    return
  }
  if (!passwordForm.newPassword || passwordForm.newPassword.length < 6) {
    ElMessage.warning('新密码至少6位')
    return
  }
  if (passwordForm.newPassword !== passwordForm.confirmPassword) {
    ElMessage.warning('两次密码不一致')
    return
  }
  passwordSaving.value = true
  try {
    await remoteChangePassword(authStore.token, passwordForm.oldPassword, passwordForm.newPassword)
    ElMessage.success('密码已修改，请重新登录')
    passwordDialogVisible.value = false
    authStore.logout()
    router.push('/login')
  } catch {
    // error shown by API
  } finally {
    passwordSaving.value = false
  }
}

authStore.init()
if (authStore.isAuthenticated && !authStore.userInfo?.name) {
  authStore.fetchProfile()
}
</script>

<style scoped lang="scss">
.app-header-inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 100%;
  user-select: none;
}
.header-left {
  display: flex;
  align-items: center;
  -webkit-app-region: drag;
}
.traffic-light-space {
  width: 78px;
  -webkit-app-region: no-drag;
  flex-shrink: 0;
}
.header-center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  -webkit-app-region: drag;
}
.app-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--el-text-color-regular);
  opacity: 0.8;
  pointer-events: none;
}
.collapse-btn {
  cursor: pointer;
  font-size: 20px;
  color: var(--el-text-color-regular);
  -webkit-app-region: no-drag;
  margin-right: 8px;
  &:hover {
    color: var(--el-color-primary);
  }
}
.theme-btn {
  cursor: pointer;
  font-size: 18px;
  color: var(--el-text-color-regular);
  -webkit-app-region: no-drag;
  &:hover {
    color: var(--el-color-primary);
  }
}
.help-btn {
  cursor: pointer;
  font-size: 18px;
  color: var(--el-text-color-regular);
  -webkit-app-region: no-drag;
  &:hover {
    color: var(--el-color-primary);
  }
}
.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
  -webkit-app-region: no-drag;
}
.user-dropdown {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
}
.user-name {
  font-size: 13px;
  color: var(--el-text-color-regular);
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-info-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  pointer-events: none;
}
.user-info-text {
  flex: 1;
  min-width: 0;
}
.user-info-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-info-mobile {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 2px;
}
.help-content {
  ul {
    margin: 0;
    padding-left: 20px;
    line-height: 2;
    color: var(--el-text-color-regular);
  }
  code {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 12px;
    background: var(--el-fill-color-light);
    color: var(--el-color-danger);
  }
}

:deep(.el-dropdown-menu__item.is-active) {
  color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}
</style>
