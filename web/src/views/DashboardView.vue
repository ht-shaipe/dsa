<template>
  <div class="dashboard">
    <div class="market-bar">
      <template v-if="marketLoading">
        <div v-for="i in 3" :key="i" class="market-item">
          <el-skeleton :rows="0" animated>
            <template #template>
              <el-skeleton-item variant="text" style="width: 50px; height: 16px; margin-right: 8px" />
              <el-skeleton-item variant="text" style="width: 60px; height: 20px; margin-right: 8px" />
              <el-skeleton-item variant="text" style="width: 55px; height: 16px" />
            </template>
          </el-skeleton>
        </div>
      </template>
      <template v-else>
        <div v-for="item in marketOverview" :key="item.name" class="market-item">
          <span class="market-name">{{ item.name }}</span>
          <span :class="['market-price', item.change >= 0 ? 'up' : 'down']">{{ formatNum(item.price, 2) }}</span>
          <span :class="['market-change', item.change >= 0 ? 'up' : 'down']">
            {{ item.change >= 0 ? '+' : '' }}{{ formatNum(item.change, 2) }}%
          </span>
        </div>
      </template>
      <div class="market-bar-right">
        <span class="market-time">{{ currentTime }}</span>
      </div>
    </div>

    <div class="stats-bar">
      <template v-if="statsLoading">
        <div v-for="i in 6" :key="i" class="stats-item">
          <span class="stats-label"><el-skeleton-item variant="text" style="width:60px;height:14px" /></span>
          <span class="stats-value"><el-skeleton-item variant="text" style="width:50px;height:22px" /></span>
        </div>
      </template>
      <template v-else>
        <div class="stats-item" title="股票池中的股票数量">
          <span class="stats-label">股票池</span>
          <span class="stats-value">{{ stats.pool.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="已有日线数据的股票数">
          <span class="stats-label">日线股票</span>
          <span class="stats-value">{{ stats.dailyStocks.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="日线数据总条数">
          <span class="stats-label">日线条数</span>
          <span class="stats-value">{{ stats.dailyTotal.toLocaleString() }}</span>
        </div>
        <div class="stats-item stats-divider"></div>
        <div class="stats-item" title="自选股数量">
          <span class="stats-label">自选股</span>
          <span class="stats-value">{{ stats.watchlist.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="持仓数量">
          <span class="stats-label">持仓</span>
          <span class="stats-value">{{ stats.positions.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="持仓总市值">
          <span class="stats-label">市值</span>
          <span :class="['stats-value', stats.pnl >= 0 ? 'up' : 'down']">{{ formatMoney(stats.totalValue) }}</span>
        </div>
        <div class="stats-item stats-divider"></div>
        <div class="stats-item" title="决策信号: 看多/看空/总计">
          <span class="stats-label">信号</span>
          <span class="stats-value">
            <span class="up">{{ stats.bullish }}</span>/<span class="down">{{ stats.bearish }}</span>/<span>{{ stats.decisions }}</span>
          </span>
        </div>
        <div class="stats-item" title="AI分析报告数">
          <span class="stats-label">分析</span>
          <span class="stats-value">{{ stats.analysis.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="回测交易数/胜率">
          <span class="stats-label">回测</span>
          <span class="stats-value">{{ stats.backtests }}<span v-if="stats.backtests" class="stats-sub"> /{{ stats.winRate.toFixed(0) }}%</span></span>
        </div>
        <div class="stats-item stats-divider"></div>
        <div class="stats-item" title="今日预警触发">
          <span class="stats-label">预警</span>
          <span class="stats-value">{{ stats.alerts.toLocaleString() }}</span>
        </div>
        <div class="stats-item" title="情报源/情报条目">
          <span class="stats-label">情报</span>
          <span class="stats-value">{{ stats.intelSources }}<span class="stats-sub"> /{{ stats.intelItems }}</span></span>
        </div>
        <div class="stats-item" title="今日LLM调用/总Token">
          <span class="stats-label">LLM</span>
          <span class="stats-value">{{ stats.llmToday }}<span class="stats-sub"> /{{ formatTokens(stats.llmTokens) }}</span></span>
        </div>
      </template>
    </div>

    <div class="watchlist-section" v-if="positions.length">
      <div class="section-header">
        <span class="section-title">持仓行情</span>
        <el-button link type="primary" @click="$router.push('/portfolio')">管理投资组合 →</el-button>
      </div>
      <PositionCardList
        :positions="positions"
        :columns="4"
        empty-text="暂无持仓"
        @click="analyzeFromWatchlist"
      />
    </div>

    <div class="watchlist-section" v-if="watchlist.length">
      <div class="section-header">
        <span class="section-title">自选股行情</span>
        <el-button link type="primary" @click="$router.push('/watchlist')">管理自选股 →</el-button>
      </div>
      <div class="watchlist-grid">
        <div v-for="row in watchlist" :key="row.stockCode || row.code" class="stock-card" @click="analyzeFromWatchlist(row)">
          <div class="stock-card-top">
            <span class="stock-card-name">{{ row.name || row.stockName || '-' }}</span>
            <span class="stock-card-code">{{ row.stockCode || row.code }}</span>
          </div>
          <div class="stock-card-bottom">
            <span :class="['stock-card-price', row.changePercent >= 0 ? 'up' : 'down']">
              <template v-if="row.close != null">{{ formatNum(row.close, 2) }}</template>
              <template v-else-if="row.price != null">{{ formatNum(row.price, 2) }}</template>
              <template v-else>-</template>
            </span>
            <span v-if="row.changePercent != null" :class="['stock-card-change', Number(row.changePercent) >= 0 ? 'up' : 'down']">
              {{ Number(row.changePercent) >= 0 ? '+' : '' }}{{ formatNum(row.changePercent, 2) }}%
            </span>
          </div>
        </div>
      </div>
    </div>


    <div class="analysis-section">
      <div class="analysis-header">
        <div class="analysis-title">
          <el-icon :size="20" color="var(--el-color-primary)"><DataAnalysis /></el-icon>
          <span>AI 智能分析</span>
          <template v-if="selectedCode">
            <el-divider direction="vertical" />
            <span class="selected-stock">{{ selectedName }}</span>
            <span class="selected-code">{{ selectedCode }}</span>
          </template>
        </div>
        <div class="analysis-actions">
          <StockAutocomplete v-model="searchText" @select="onStockSelect" style="width: 240px" />
          <el-button type="primary" :loading="analysisStore.isAnalyzing" @click="runAnalysis" :disabled="!selectedCode">
            <el-icon><CaretRight /></el-icon>
            {{ analysisStore.isAnalyzing ? '分析中...' : '开始分析' }}
          </el-button>
        </div>
      </div>

      <div class="analysis-body">
        <template v-if="analysisStore.isAnalyzing && analysisStore.streamingText">
          <div class="stream-container">
            <div v-if="analysisStore.streamStatus" class="stream-status">
              <el-icon class="is-loading"><Loading /></el-icon>
              <span>{{ analysisStore.streamStatus }}</span>
            </div>
            <div class="stream-text">
              <pre>{{ analysisStore.streamingText }}</pre>
            </div>
          </div>
        </template>
        <template v-else-if="analysisStore.isAnalyzing">
          <div class="stream-container">
            <div class="stream-status">
              <el-icon class="is-loading"><Loading /></el-icon>
              <span>{{ analysisStore.streamStatus || '准备分析...' }}</span>
            </div>
          </div>
        </template>
        <template v-else-if="analysisStore.currentReport">
          <div class="report-summary">
            <div class="score-area">
              <ScoreGauge :score="analysisStore.currentReport.sentimentScore || 0" :size="120" />
            </div>
            <div class="summary-info">
              <el-tag :type="actionType" size="large" round>{{ actionLabel }}</el-tag>
              <div v-if="analysisStore.currentReport.dataAsOf" class="meta-item">
                <span class="meta-label">数据基准</span>
                <span class="meta-value" style="font-size:12px;color:var(--el-text-color-secondary)">{{ analysisStore.currentReport.dataAsOf }}</span>
              </div>
              <div v-if="analysisStore.currentReport.targetPrice" class="meta-item">
                <span class="meta-label">目标价</span>
                <span class="meta-value">{{ analysisStore.currentReport.targetPrice }}</span>
              </div>
              <div v-if="analysisStore.currentReport.stopLossPrice" class="meta-item">
                <span class="meta-label">止损价</span>
                <span class="meta-value danger">{{ analysisStore.currentReport.stopLossPrice }}</span>
              </div>
            </div>
          </div>
          <el-scrollbar class="report-content" max-height="500px">
            <MarkdownRenderer :content="analysisStore.currentReport.markdown || analysisStore.currentReport.text || ''" />
          </el-scrollbar>
        </template>
        <div v-else class="report-empty">
          <el-icon :size="40" color="var(--el-fill-color)"><DataAnalysis /></el-icon>
          <p v-if="selectedCode">已选择 {{ selectedName }}（{{ selectedCode }}），点击「开始分析」</p>
          <p v-else>选择股票并点击分析，AI 将生成完整投资分析报告</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useTradingInterval } from '@/composables/useTradingInterval'
import { ElMessage } from 'element-plus'
import { Loading } from '@element-plus/icons-vue'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import ScoreGauge from '@/components/common/ScoreGauge.vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'
import PositionCardList from '@/components/common/PositionCardList.vue'
import { marketApi } from '@/api/market'
import { stockApi } from '@/api/stock'
import { portfolioApi } from '@/api/portfolio'
import { analysisApi } from '@/api/analysis'
import { systemApi } from '@/api/system'
import { useAnalysisStore } from '@/stores/analysis'
import { formatNum, formatMoney, formatTokens } from '@/utils/format'

const analysisStore = useAnalysisStore()
const marketOverview = ref<any[]>([])
const marketLoading = ref(true)
const watchlist = ref<any[]>([])
const positions = ref<any[]>([])
const stats = ref({
  pool: 0, dailyStocks: 0, dailyTotal: 0, latestDate: '',
  watchlist: 0, positions: 0, totalValue: 0, pnl: 0,
  decisions: 0, bullish: 0, bearish: 0, avgScore: 0,
  analysis: 0, backtests: 0, winRate: 0,
  alerts: 0, alertRules: 0,
  intelSources: 0, intelItems: 0,
  llmToday: 0, llmTokens: 0,
  newsCount: 0, syncRunning: false,
})
const statsLoading = ref(true)
const selectedCode = ref('')
const selectedName = ref('')
const searchText = ref('')
const currentTime = ref('')
let streamAbort: AbortController | null = null

function updateTime() {
  const now = new Date()
  currentTime.value = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`
}

const tradingTimer = useTradingInterval(() => {
  loadMarketOverview()
  loadWatchlist()
  loadPositions()
}, 10000)

let clockTimer: ReturnType<typeof setInterval> | null = null

const actionLabel = computed(() => {
  const dt = analysisStore.currentReport?.decisionType || ''
  return dt === 'buy' ? '买入' : dt === 'sell' ? '卖出' : dt === 'hold' ? '持有' : dt || '分析'
})
const actionType = computed(() => {
  const dt = analysisStore.currentReport?.decisionType || ''
  return dt === 'buy' ? 'success' : dt === 'sell' ? 'danger' : 'warning'
})

function onStockSelect(code: string, name: string) {
  selectedCode.value = code
  selectedName.value = name
  searchText.value = code
}

async function runAnalysis() {
  if (!selectedCode.value) return
  if (streamAbort) {
    streamAbort.abort()
    streamAbort = null
  }
  analysisStore.setAnalyzing(true)
  analysisStore.clearReport()

  streamAbort = analysisApi.analyzeStream(selectedCode.value, selectedName.value, {
    onStatus(content) {
      analysisStore.setStreamStatus(content)
    },
    onText(chunk) {
      analysisStore.appendStreamText(chunk)
    },
    onReport(data) {
      if (data.type === 'report') {
        analysisStore.setReport(data)
      } else if (data.type === 'raw') {
        analysisStore.setReport({
          markdown: data.content,
          text: data.content,
          sentimentScore: 0,
          decisionType: '',
          operationAdvice: '',
          parseError: data.parse_error,
        })
      }
    },
    onError(message) {
      ElMessage.error(message)
    },
    onDone() {
      analysisStore.setAnalyzing(false)
      analysisStore.clearStreamState()
      streamAbort = null
    },
  })
}

async function loadMarketOverview() {
  try {
    const d: any = await marketApi.overview()
    marketOverview.value = [
      { name: d.sh?.name || '上证指数', price: d.sh?.price || 0, change: d.sh?.changePercent || d.sh?.change_pct || 0 },
      { name: d.sz?.name || '深证成指', price: d.sz?.price || 0, change: d.sz?.changePercent || d.sz?.change_pct || 0 },
      { name: d.cy?.name || '创业板指', price: d.cy?.price || 0, change: d.cy?.changePercent || d.cy?.change_pct || 0 },
    ]
  } catch { /* ignore */ }
  marketLoading.value = false
}

async function loadWatchlist() {
  try {
    const data: any = await stockApi.watchlist()
    watchlist.value = Array.isArray(data) ? data : []
  } catch { /* ignore */ }
}

async function loadPositions() {
  try {
    const data: any = await portfolioApi.positions()
    positions.value = Array.isArray(data) ? data : []
  } catch { /* ignore */ }
}

async function loadDashboardStats() {
  try {
    const d: any = await systemApi.dashboardStats()
    stats.value = {
      pool: d?.pool?.count || 0,
      dailyStocks: d?.daily?.stockCount || 0,
      dailyTotal: d?.daily?.totalCount || 0,
      latestDate: d?.daily?.latestDate || '',
      watchlist: d?.watchlist?.count || 0,
      positions: d?.portfolio?.positionCount || 0,
      totalValue: d?.portfolio?.totalValue || 0,
      pnl: d?.portfolio?.totalPnl || 0,
      decisions: d?.decision?.total || 0,
      bullish: d?.decision?.bullish || 0,
      bearish: d?.decision?.bearish || 0,
      avgScore: d?.decision?.avgScore || 0,
      analysis: d?.analysis?.count || 0,
      backtests: d?.backtest?.count || 0,
      winRate: d?.backtest?.winRate || 0,
      alerts: d?.alert?.todayTriggered || 0,
      alertRules: d?.alert?.ruleCount || 0,
      intelSources: d?.intelligence?.sourceCount || 0,
      intelItems: d?.intelligence?.itemCount || 0,
      llmToday: d?.llm?.todayCalls || 0,
      llmTokens: d?.llm?.totalTokens || 0,
      newsCount: d?.news?.count || 0,
      syncRunning: d?.sync?.running || false,
    }
  } catch { /* ignore */ }
  statsLoading.value = false
}



function analyzeFromWatchlist(row: any) {
  selectedCode.value = row.stockCode || row.code || ''
  selectedName.value = row.name || row.stockName || ''
  searchText.value = selectedCode.value
}

onMounted(() => {
  tradingTimer.start()
  updateTime()
  clockTimer = setInterval(updateTime, 1000)
  loadDashboardStats()
})

onUnmounted(() => {
  tradingTimer.stop()
  if (streamAbort) { streamAbort.abort(); streamAbort = null }
  if (clockTimer) { clearInterval(clockTimer); clockTimer = null }
})
</script>

<style scoped lang="scss">
.dashboard {
  max-width: 1200px;
  margin: 0 auto;
}

.up { color: #f56c6c; }
.down { color: #67c23a; }

.market-bar {
  display: flex;
  align-items: center;
  gap: 32px;
  padding: 12px 20px;
  background: var(--el-bg-color);
  border-radius: 8px;
  margin-bottom: 10px;
  border: 1px solid var(--el-border-color-lighter);
}
.market-item {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.market-name { font-size: 13px; color: var(--el-text-color-secondary); }
.market-price { font-size: 16px; font-weight: 600; font-variant-numeric: tabular-nums; }
.market-change { font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; }
.market-bar-right { margin-left: auto; }
.market-time { font-size: 12px; color: var(--el-text-color-placeholder); font-variant-numeric: tabular-nums; }

.stats-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 20px;
  background: var(--el-bg-color);
  border-radius: 8px;
  margin-bottom: 10px;
  border: 1px solid var(--el-border-color-lighter);
  flex-wrap: wrap;
}
.stats-item {
  display: flex;
  align-items: center;
  gap: 6px;
}
.stats-label { font-size: 12px; color: var(--el-text-color-secondary); white-space: nowrap; }
.stats-value { font-size: 16px; font-weight: 600; color: var(--el-color-primary); font-variant-numeric: tabular-nums; white-space: nowrap; }
.stats-value .up { color: #f56c6c; }
.stats-value .down { color: #67c23a; }
.stats-sub { font-size: 12px; font-weight: 400; color: var(--el-text-color-secondary); }
.stats-divider {
  width: 1px;
  height: 18px;
  background: var(--el-border-color-lighter);
  padding: 0;
  gap: 0;
}

.watchlist-section {
  background: var(--el-bg-color);
  border-radius: 8px;
  border: 1px solid var(--el-border-color-lighter);
  padding: 16px 20px;
  margin-bottom: 20px;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.section-title { font-size: 15px; font-weight: 600; }
.watchlist-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
}
.stock-card {
  padding: 12px;
  border-radius: 6px;
  border: 1px solid var(--el-border-color-extra-light);
  cursor: pointer;
  transition: all 0.2s;
  &:hover {
    border-color: var(--el-color-primary-light-5);
    background: var(--el-color-primary-light-9);
  }
}
.stock-card-top {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}
.stock-card-name {
  font-size: 14px;
  font-weight: 500;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.stock-card-code { font-size: 12px; color: var(--el-text-color-secondary); }

.stock-card-bottom {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}
.stock-card-price { font-size: 16px; font-weight: 600; font-variant-numeric: tabular-nums; }
.stock-card-change { font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; }

.analysis-section {
  background: var(--el-bg-color);
  border-radius: 8px;
  border: 1px solid var(--el-border-color-lighter);
}
.analysis-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.analysis-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
}
.selected-stock { font-weight: 500; }
.selected-code { font-size: 13px; color: var(--el-text-color-secondary); }
.analysis-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.analysis-body {
  padding: 20px;
}

.report-summary {
  display: flex;
  align-items: center;
  gap: 24px;
  margin-bottom: 20px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--el-border-color-extra-light);
}
.score-area { flex-shrink: 0; }
.summary-info {
  display: flex;
  align-items: center;
  gap: 20px;
}
.meta-item {
  display: flex;
  align-items: baseline;
  gap: 6px;
}
.meta-label { font-size: 12px; color: var(--el-text-color-placeholder); }
.meta-value { font-size: 16px; font-weight: 600; &.danger { color: var(--el-color-danger); } }

.report-content { line-height: 1.7; }

.stream-container {
  padding: 20px;
}
.stream-status {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  margin-bottom: 16px;
}
.stream-text {
  background: var(--el-fill-color-lighter);
  border-radius: 8px;
  padding: 16px;
  max-height: 500px;
  overflow-y: auto;
  pre {
    margin: 0;
    font-family: inherit;
    font-size: 14px;
    line-height: 1.7;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--el-text-color-primary);
  }
}

.report-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 40px 20px;
  p {
    margin-top: 12px;
    font-size: 14px;
    color: var(--el-text-color-placeholder);
  }
}
</style>
