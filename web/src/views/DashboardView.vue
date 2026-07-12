<template>
  <div class="dashboard">
    <el-row :gutter="20">
      <el-col :span="6" v-for="item in marketOverview" :key="item.name">
        <el-card shadow="hover" class="overview-card">
          <div class="overview-name">{{ item.name }}</div>
          <div class="overview-price">{{ item.price }}</div>
          <div :class="['overview-change', item.change >= 0 ? 'up' : 'down']">
            {{ item.change >= 0 ? '+' : '' }}{{ item.change?.toFixed(2) }}%
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>股票分析</template>
          <StockAutocomplete v-model="searchText" @select="onStockSelect" />
          <div style="margin-top: 12px">
            <el-button type="primary" :loading="analysisStore.isAnalyzing" @click="runAnalysis" :disabled="!selectedCode">
              开始分析
            </el-button>
          </div>
        </el-card>
      </el-col>
      <el-col :span="16">
        <el-card shadow="hover" v-if="analysisStore.currentReport">
          <template #header>
            <div style="display:flex;justify-content:space-between;align-items:center">
              <span>分析报告 - {{ selectedCode }} {{ selectedName }}</span>
              <el-tag :type="actionType" size="large">{{ actionLabel }}</el-tag>
            </div>
          </template>
          <div style="display:flex;gap:20px">
            <ScoreGauge :score="analysisStore.currentReport.sentimentScore || 0" :size="120" />
            <div style="flex:1">
              <MarkdownRenderer :content="analysisStore.currentReport.markdown || analysisStore.currentReport.text || ''" />
            </div>
          </div>
        </el-card>
        <el-card shadow="hover" v-else>
          <el-empty description="选择股票并开始分析" />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <div style="display:flex;justify-content:space-between;align-items:center">
              <span>自选股行情</span>
              <el-button link type="primary" @click="$router.push('/watchlist')">管理自选股</el-button>
            </div>
          </template>
          <el-table :data="watchlist" stripe style="width:100%">
            <el-table-column label="代码" width="100">
              <template #default="{ row }">
                <span style="font-weight:500">{{ row.stockCode || row.code }}</span>
              </template>
            </el-table-column>
            <el-table-column label="名称" width="120">
              <template #default="{ row }">{{ row.name || row.stockName || '-' }}</template>
            </el-table-column>
            <el-table-column label="现价" width="100">
              <template #default="{ row }">
                <span v-if="row.close != null">{{ Number(row.close).toFixed(2) }}</span>
                <span v-else-if="row.price != null">{{ Number(row.price).toFixed(2) }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="涨跌幅" width="100">
              <template #default="{ row }">
                <span v-if="row.changePercent != null" :class="Number(row.changePercent) >= 0 ? 'up' : 'down'">
                  {{ Number(row.changePercent) >= 0 ? '+' : '' }}{{ Number(row.changePercent).toFixed(2) }}%
                </span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="成交量" width="120">
              <template #default="{ row }">{{ row.volume ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ row }">
                <el-button link type="primary" @click="analyzeFromWatchlist(row)">分析</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!watchlist.length" description="暂无自选股，前往自选股管理添加" />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useTradingInterval } from '@/composables/useTradingInterval'
import { ElMessage } from 'element-plus'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import ScoreGauge from '@/components/common/ScoreGauge.vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'
import { marketApi } from '@/api/market'
import { stockApi } from '@/api/stock'
import { analysisApi } from '@/api/analysis'
import { useAnalysisStore } from '@/stores/analysis'

const analysisStore = useAnalysisStore()
const marketOverview = ref<any[]>([])
const watchlist = ref<any[]>([])
const selectedCode = ref('')
const selectedName = ref('')
const searchText = ref('')

const tradingTimer = useTradingInterval(() => {
  loadMarketOverview()
  loadWatchlist()
}, 10000)

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
  analysisStore.setAnalyzing(true)
  try {
    const res: any = await analysisApi.analyze(selectedCode.value, selectedName.value)
    const data = res.data || res
    if (data) {
      analysisStore.setReport(data)
    } else {
      ElMessage.warning('分析完成但未返回报告数据')
    }
  } catch (e: any) {
    const msg = e?.response?.data?.message || e?.message || '分析失败'
    ElMessage.error(msg)
  } finally {
    analysisStore.setAnalyzing(false)
  }
}

async function loadMarketOverview() {
  try {
    const res: any = await marketApi.overview()
    const d = res.data || {}
    marketOverview.value = [
      { name: d.sh?.name || '上证指数', price: d.sh?.price || 0, change: d.sh?.changePercent || d.sh?.change_pct || 0 },
      { name: d.sz?.name || '深证成指', price: d.sz?.price || 0, change: d.sz?.changePercent || d.sz?.change_pct || 0 },
      { name: d.cy?.name || '创业板指', price: d.cy?.price || 0, change: d.cy?.changePercent || d.cy?.change_pct || 0 },
    ]
  } catch { /* ignore */ }
}

async function loadWatchlist() {
  try {
    const res: any = await stockApi.watchlist()
    watchlist.value = res.data || []
  } catch { /* ignore */ }
}

function analyzeFromWatchlist(row: any) {
  selectedCode.value = row.stockCode || row.code || ''
  selectedName.value = row.name || row.stockName || ''
  searchText.value = selectedCode.value
}

onMounted(() => {
  tradingTimer.start()
})

onUnmounted(() => {
  tradingTimer.stop()
})
</script>

<style scoped lang="scss">
.overview-card {
  text-align: center;
}
.overview-name { font-size: 14px; color: var(--el-text-color-secondary); }
.overview-price { font-size: 24px; font-weight: bold; margin: 8px 0; }
.overview-change { font-size: 16px; font-weight: 500; }
.up { color: #f56c6c; }
.down { color: #67c23a; }
</style>
