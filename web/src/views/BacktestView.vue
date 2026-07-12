<template>
  <div class="backtest-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>回测分析</span>
          <el-tag type="info" size="small">评估历史决策信号的实际表现</el-tag>
        </div>
      </template>
      <el-steps :active="workflowStep" finish-status="success" align-center style="max-width:700px;margin:0 auto 24px">
        <el-step title="产生信号" description="分析股票后自动生成决策信号" />
        <el-step title="选择信号" description="从信号列表中选择待回测信号" />
        <el-step title="执行回测" description="评估信号在历史数据中的表现" />
        <el-step title="查看结果" description="胜率、收益率、最大回撤等" />
      </el-steps>

      <el-collapse v-model="guideExpanded" style="margin-bottom:16px">
        <el-collapse-item title="如何使用回测分析？" name="guide">
          <el-timeline>
            <el-timeline-item type="primary" size="large">
              <h4 style="margin:0 0 4px">1. 先分析股票，产生决策信号</h4>
              <p style="margin:0;color:var(--el-text-color-secondary)">
                前往<el-link type="primary" @click="$router.push('/')" style="font-size:inherit">首页</el-link>或<el-link type="primary" @click="$router.push('/chat')" style="font-size:inherit">对话</el-link>，选择一只股票点击"开始分析"。系统分析后会自动生成决策信号（买入/卖出/持有建议）。
              </p>
            </el-timeline-item>
            <el-timeline-item type="warning" size="large">
              <h4 style="margin:0 0 4px">2. 选择信号执行回测</h4>
              <p style="margin:0;color:var(--el-text-color-secondary)">
                在下方"待回测信号"列表中找到想验证的信号，点击右侧<strong>「回测」</strong>按钮。也可以在<el-link type="primary" @click="$router.push('/decisions')" style="font-size:inherit">决策信号</el-link>页点"回测验证"跳转过来。
              </p>
            </el-timeline-item>
            <el-timeline-item type="success" size="large">
              <h4 style="margin:0 0 4px">3. 查看回测结果</h4>
              <p style="margin:0;color:var(--el-text-color-secondary)">
                回测会取信号日期之后的真实K线数据，计算收益率、最大回撤、是否触发止损/止盈等。结果中：<strong style="color:#67c23a">命中</strong>=方向判断正确，<strong style="color:#f56c6c">未中</strong>=方向判断错误。
              </p>
            </el-timeline-item>
            <el-timeline-item type="info" size="large">
              <h4 style="margin:0 0 4px">4. 关注统计指标</h4>
              <p style="margin:0;color:var(--el-text-color-secondary)">
                右上角统计卡片展示整体胜率、平均收益等。回测信号越多，统计越有参考价值。建议积累 20+ 信号后关注胜率指标。
              </p>
            </el-timeline-item>
          </el-timeline>
        </el-collapse-item>
      </el-collapse>

      <el-alert v-if="!recentSignals.length && !guideExpanded.length" title="还没有决策信号，无法执行回测" type="info" show-icon :closable="false">
        <template #default>
          回测需要一个"决策信号"作为输入。请先去<el-link type="primary" @click="$router.push('/')">首页分析一只股票</el-link>，系统会自动生成信号，然后回到这里执行回测。
        </template>
      </el-alert>
    </el-card>

    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="16">
        <el-card shadow="hover">
          <template #header>
            <div style="display:flex;justify-content:space-between;align-items:center">
              <span>待回测信号
                <el-tooltip content="系统分析股票后自动生成的买卖建议。点击任意行的「回测」按钮即可验证该信号。" placement="top">
                  <el-icon style="color:var(--el-text-color-secondary);cursor:help"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
              <div style="display:flex;gap:8px;align-items:center">
                <StockAutocomplete v-model="signalCodeFilter" @select="onSignalFilterSelect" placeholder="按股票筛选" style="width:200px" />
                <el-button @click="loadRecentSignals">刷新</el-button>
              </div>
            </div>
          </template>
          <el-table :data="recentSignals" stripe style="width:100%" v-loading="signalsLoading" size="small" highlight-current-row @current-change="onSignalSelect">
            <el-table-column prop="id" label="ID" width="60" />
            <el-table-column label="代码" width="90">
              <template #default="{ row }">{{ row.stockCode || row.stock_code }}</template>
            </el-table-column>
            <el-table-column label="名称" width="100">
              <template #default="{ row }">{{ row.stockName || row.stock_name }}</template>
            </el-table-column>
            <el-table-column label="操作" width="70">
              <template #default="{ row }">
                <el-tag :type="actionTagType(row.action || row.decisionType)" size="small">{{ actionLabel(row.action || row.decisionType) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="评分" width="70" align="center">
              <template #default="{ row }">{{ row.sentimentScore || row.sentiment_score || '-' }}</template>
            </el-table-column>
            <el-table-column label="置信度" width="80">
              <template #default="{ row }">{{ row.confidenceLevel || row.confidence_level || '-' }}</template>
            </el-table-column>
            <el-table-column label="信号日期" width="110">
              <template #default="{ row }">{{ formatDate(row.signalDate || row.signal_date || row.createTime || row.create_time) }}</template>
            </el-table-column>
            <el-table-column label="状态" width="80">
              <template #default="{ row }">
                <el-tooltip :content="row.status === 4 ? '已执行过回测，可重复回测' : '尚未回测，点击右侧「回测」按钮验证'" placement="top">
                  <el-tag :type="signalStatusType(row.status)" size="small">{{ signalStatusLabel(row.status) }}</el-tag>
                </el-tooltip>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" size="small" @click="runEvaluateForSignal(row)" :disabled="row._evaluating" :loading="row._evaluating">
                  回测
                </el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!recentSignals.length && !signalsLoading" description="暂无可回测信号">
            <template #extra>
              <el-button type="primary" size="small" @click="$router.push('/')">去分析股票</el-button>
            </template>
          </el-empty>
        </el-card>
      </el-col>

      <el-col :span="8">
        <el-card shadow="hover" style="margin-bottom: 16px">
          <template #header>快速回测
            <el-tooltip content="如果你知道信号ID，可以直接输入执行回测。信号ID可在左侧信号列表或「决策信号」页查看。" placement="top">
              <el-icon style="color:var(--el-text-color-secondary);cursor:help;margin-left:4px"><QuestionFilled /></el-icon>
            </el-tooltip>
          </template>
          <el-form label-width="70px" size="small">
            <el-form-item label="信号ID">
              <el-input-number v-model="signalId" :min="1" style="width:100%" placeholder="输入信号ID" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="evaluating" :disabled="!signalId" @click="runEvaluate" style="width:100%">
                执行回测
              </el-button>
            </el-form-item>
            <el-form-item>
              <el-button :loading="batching" @click="runBatch" style="width:100%">
                批量回测
              </el-button>
              <div style="font-size:12px;color:var(--el-text-color-placeholder);margin-top:4px">
                自动选取最近50条未回测信号批量评估
              </div>
            </el-form-item>
          </el-form>
          <el-alert v-if="evalResult" :title="evalTitle" :type="evalResult.outcome === 'win' ? 'success' : evalResult.outcome === 'loss' ? 'error' : 'info'" show-icon :closable="false" style="margin-top:8px">
            <div>股票: {{ evalResult.stockCode }} {{ evalResult.stockName }}</div>
            <div>方向: {{ actionLabel(evalResult.decisionAction) }} → {{ evalResult.directionExpected === 'up' ? '看涨' : '看跌' }}</div>
            <div>收益率: {{ formatPct(evalResult.stockReturnPct) }}</div>
            <div>最大回撤: {{ formatPct(evalResult.maxDrawdown) }}</div>
            <div>方向正确: {{ evalResult.directionCorrect ? '是' : '否' }}</div>
          </el-alert>
        </el-card>

        <el-row :gutter="12">
          <el-col :span="12" v-for="card in perfCards" :key="card.label">
            <el-card shadow="hover" class="perf-card">
              <el-statistic :title="card.label" :value="card.value" :precision="card.precision" :suffix="card.suffix" />
            </el-card>
          </el-col>
        </el-row>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>回测结果
                <el-tooltip content="回测用信号日之后的真实行情验证：买入信号看后续是否上涨，卖出信号看后续是否下跌。命中=方向正确，未中=方向错误。" placement="top">
                  <el-icon style="color:var(--el-text-color-secondary);cursor:help"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
          <div style="display:flex;gap:8px;align-items:center">
            <el-select v-model="listOutcomeFilter" placeholder="筛选结果" clearable style="width:120px" @change="loadList">
              <el-option label="命中(win)" value="win" />
              <el-option label="未中(loss)" value="loss" />
              <el-option label="中性" value="neutral" />
            </el-select>
            <el-input v-model="listCodeFilter" placeholder="按代码筛选" clearable style="width:140px" @clear="loadList" @keyup.enter="loadList">
              <template #append>
                <el-button @click="loadList">查询</el-button>
              </template>
            </el-input>
          </div>
        </div>
      </template>
      <el-table :data="backtestList" stripe style="width:100%" v-loading="listLoading">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column label="代码" width="90">
          <template #default="{ row }">{{ row.stockCode || row.stock_code || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="70">
          <template #default="{ row }">{{ actionLabel(row.decisionAction || row.decision_action || row.action) }}</template>
        </el-table-column>
        <el-table-column label="入场价" width="85">
          <template #default="{ row }">{{ Number(row.simulatedEntry || row.startPrice || row.start_price || 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="收益%" width="95">
          <template #default="{ row }">
            <span :class="pnlClass(row.stockReturnPct || row.stock_return_pct)">
              {{ formatPct(row.stockReturnPct || row.stock_return_pct) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="最大回撤" width="95">
          <template #default="{ row }">
            <span style="color:#f56c6c">{{ formatPct(row.maxDrawdown || row.max_drawdown) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="结果" width="75">
          <template #default="{ row }">
            <el-tag :type="outcomeTag(row.outcome)" size="small">{{ outcomeLabel(row.outcome) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="方向" width="70">
          <template #default="{ row }">
            <el-tag :type="row.directionCorrect || row.direction_correct ? 'success' : 'danger'" size="small">
              {{ row.directionCorrect || row.direction_correct ? '正确' : '错误' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="信号日期" width="105">
          <template #default="{ row }">{{ formatDate(row.signalDate || row.signal_date) }}</template>
        </el-table-column>
        <el-table-column label="创建时间" width="105">
          <template #default="{ row }">{{ formatDate(row.createTime || row.create_time) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="60" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="viewDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!backtestList.length && !listLoading" description="暂无回测记录，请先对信号执行回测" />
    </el-card>

    <el-dialog v-model="detailVisible" title="回测详情" width="700px">
      <el-descriptions :column="2" border v-if="detailData">
        <el-descriptions-item label="股票代码">{{ detailData.stockCode || detailData.stock_code || '-' }}</el-descriptions-item>
        <el-descriptions-item label="决策操作">{{ actionLabel(detailData.decisionAction || detailData.decision_action) }}</el-descriptions-item>
        <el-descriptions-item label="信号日期">{{ formatDate(detailData.signalDate || detailData.signal_date) }}</el-descriptions-item>
        <el-descriptions-item label="评估窗口">{{ detailData.evalWindowDays || detailData.eval_window_days || '-' }}天</el-descriptions-item>
        <el-descriptions-item label="起始价">{{ Number(detailData.startPrice || detailData.start_price || 0).toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="终价">{{ Number(detailData.endClose || detailData.end_close || 0).toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="最高价">{{ Number(detailData.maxHigh || detailData.max_high || 0).toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="最低价">{{ Number(detailData.minLow || detailData.min_low || 0).toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="收益率">
          <span :class="pnlClass(detailData.stockReturnPct || detailData.stock_return_pct)">{{ formatPct(detailData.stockReturnPct || detailData.stock_return_pct) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="最大回撤">
          <span style="color:#f56c6c">{{ formatPct(detailData.maxDrawdown || detailData.max_drawdown) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="预期方向">{{ detailData.directionExpected || detailData.direction_expected || '-' }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="outcomeTag(detailData.outcome)">{{ outcomeLabel(detailData.outcome) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="方向正确">
          <el-tag :type="(detailData.directionCorrect || detailData.direction_correct) ? 'success' : 'danger'">{{ (detailData.directionCorrect || detailData.direction_correct) ? '是' : '否' }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="触发止损">{{ (detailData.hitStopLoss || detailData.hit_stop_loss) ? '是' : '否' }}</el-descriptions-item>
        <el-descriptions-item label="止损价">{{ Number(detailData.stopLossPrice || detailData.stop_loss_price || 0).toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="触发止盈">{{ (detailData.hitTakeProfit || detailData.hit_take_profit) ? '是' : '否' }}</el-descriptions-item>
        <el-descriptions-item label="止盈价">{{ Number(detailData.takeProfitPrice || detailData.take_profit_price || 0).toFixed(2) }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { backtestApi } from '@/api/backtest'
import { decisionApi } from '@/api/decision'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import { QuestionFilled } from '@element-plus/icons-vue'

const route = useRoute()
const signalId = ref(0)
const evaluating = ref(false)
const batching = ref(false)
const evalResult = ref<Record<string, any> | null>(null)
const recentSignals = ref<any[]>([])
const signalsLoading = ref(false)
const signalCodeFilter = ref('')
const backtestList = ref<any[]>([])
const listLoading = ref(false)
const listCodeFilter = ref('')
const listOutcomeFilter = ref('')
const summaryData = ref<Record<string, any>>({})
const detailVisible = ref(false)
const detailData = ref<Record<string, any> | null>(null)
const guideExpanded = ref<string[]>([])

const workflowStep = computed(() => {
  if (evalResult.value) return 4
  if (recentSignals.value.length) return 2
  return 1
})

const evalTitle = computed(() => {
  if (!evalResult.value) return ''
  const r = evalResult.value
  return `${r.stockCode || ''} ${r.stockName || ''} - ${actionLabel(r.decisionAction)}`
})

const perfCards = computed(() => {
  const s = summaryData.value
  return [
    { label: '总交易数', value: s.totalTrades || 0, precision: 0, suffix: '次' },
    { label: '胜率', value: s.winRate || 0, precision: 1, suffix: '%' },
    { label: '平均收益', value: s.avgReturn || 0, precision: 2, suffix: '%' },
    { label: '最大回撤', value: s.maxDrawdown || 0, precision: 2, suffix: '%' },
  ]
})

function actionTagType(action: string) {
  return action === 'buy' ? 'success' : action === 'sell' ? 'danger' : 'warning'
}

function actionLabel(action: string | undefined) {
  const map: Record<string, string> = { buy: '买入', sell: '卖出', hold: '持有', add: '加仓', reduce: '减仓', watch: '关注', avoid: '回避', up: '看涨', down: '看跌' }
  return map[action || ''] || action || '-'
}

function signalStatusType(status: number | string) {
  if (status === 1 || status === 'pending') return 'warning'
  if (status === 2 || status === 'accepted') return 'success'
  if (status === 3 || status === 'rejected') return 'danger'
  if (status === 4 || status === 'evaluated') return 'info'
  return 'info'
}

function signalStatusLabel(status: number | string) {
  const map: Record<string, string> = { '1': '待评估', '2': '已采纳', '3': '已拒绝', '4': '已回测' }
  return map[String(status)] || String(status || '-')
}

function pnlClass(val: number | undefined) {
  const v = Number(val || 0)
  return v > 0 ? 'pnl-up' : v < 0 ? 'pnl-down' : ''
}

function formatPct(val: number | undefined) {
  const v = Number(val || 0)
  return (v >= 0 ? '+' : '') + v.toFixed(2) + '%'
}

function formatDate(val: string | undefined) {
  if (!val) return '-'
  return String(val).slice(0, 10)
}

function outcomeTag(outcome: string) {
  return outcome === 'win' ? 'success' : outcome === 'loss' ? 'danger' : 'info'
}

function outcomeLabel(outcome: string) {
  const map: Record<string, string> = { win: '命中', loss: '未中', neutral: '中性' }
  return map[outcome] || outcome || '-'
}

function onSignalFilterSelect(code: string) {
  signalCodeFilter.value = code
  loadRecentSignals()
}

function onSignalSelect(row: any) {
  if (row) signalId.value = row.id
}

async function loadRecentSignals() {
  signalsLoading.value = true
  try {
    const params: Record<string, any> = { limit: 30 }
    if (signalCodeFilter.value) params.code = signalCodeFilter.value
    const res: any = await decisionApi.list(params as any)
    recentSignals.value = (res.data || []).map((s: any) => ({ ...s, _evaluating: false }))
    if (!recentSignals.value.length && !guideExpanded.value.length) {
      guideExpanded.value = ['guide']
    }
  } catch { /* ignore */ }
  finally { signalsLoading.value = false }
}

async function runEvaluate() {
  if (!signalId.value) {
    ElMessage.warning('请选择或输入信号ID')
    return
  }
  evaluating.value = true
  try {
    const res: any = await backtestApi.evaluate(signalId.value)
    evalResult.value = res.data || null
    if (res.data?.duplicate) {
      ElMessage.info('该信号已有回测结果')
    } else {
      ElMessage.success('回测完成')
    }
    loadList()
    loadSummary()
    loadRecentSignals()
  } catch (e: any) {
    ElMessage.error(e?.message || '回测失败')
  } finally {
    evaluating.value = false
  }
}

async function runEvaluateForSignal(row: any) {
  signalId.value = row.id
  row._evaluating = true
  try {
    const res: any = await backtestApi.evaluate(row.id)
    evalResult.value = res.data || null
    if (res.data?.duplicate) {
      ElMessage.info('该信号已有回测结果')
    } else {
      ElMessage.success('回测完成')
    }
    loadList()
    loadSummary()
  } catch (e: any) {
    ElMessage.error(e?.message || '回测失败')
  } finally {
    row._evaluating = false
  }
}

async function runBatch() {
  batching.value = true
  try {
    const res: any = await backtestApi.evaluateBatch(50)
    const d = res.data || {}
    ElMessage.success(`批量回测完成: ${d.evaluatedCount || 0}条成功, ${d.errors?.length || 0}条失败`)
    loadList()
    loadSummary()
    loadRecentSignals()
  } catch (e: any) {
    ElMessage.error(e?.message || '批量回测失败')
  } finally {
    batching.value = false
  }
}

async function loadList() {
  listLoading.value = true
  try {
    const params: Record<string, any> = {}
    if (listCodeFilter.value) params.code = listCodeFilter.value
    if (listOutcomeFilter.value) params.outcome = listOutcomeFilter.value
    const res: any = await backtestApi.list(params)
    backtestList.value = res.data || []
  } catch { /* ignore */ }
  finally { listLoading.value = false }
}

async function loadSummary() {
  try {
    const res: any = await backtestApi.summary(listCodeFilter.value ? { code: listCodeFilter.value } : undefined)
    summaryData.value = res.data || {}
  } catch { /* ignore */ }
}

async function viewDetail(row: Record<string, any>) {
  try {
    const res: any = await backtestApi.detail(row.id)
    detailData.value = res.data || row
  } catch {
    detailData.value = row
  }
  detailVisible.value = true
}

onMounted(() => {
  const qSid = route.query.signalId
  if (qSid && Number(qSid) > 0) {
    signalId.value = Number(qSid)
    runEvaluate()
  }
  loadRecentSignals()
  loadList()
  loadSummary()
})
</script>

<style scoped lang="scss">
.perf-card {
  text-align: center;
  margin-bottom: 12px;
}
.pnl-up {
  color: #f56c6c;
  font-weight: 500;
}
.pnl-down {
  color: #67c23a;
  font-weight: 500;
}
</style>
