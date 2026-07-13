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
      <el-tooltip content="使用帮助" placement="bottom">
        <el-icon class="help-btn" @click="helpDrawer = true"><QuestionFilled /></el-icon>
      </el-tooltip>
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
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { QuestionFilled } from '@element-plus/icons-vue'

const appStore = useAppStore()
const authStore = useAuthStore()
const router = useRouter()

const isDark = computed({
  get: () => appStore.isDark,
  set: () => appStore.toggleTheme(),
})

const helpDrawer = ref(false)
const helpActive = ref(['quick'])

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
.help-btn {
  cursor: pointer;
  font-size: 18px;
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
</style>
