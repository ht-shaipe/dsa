<template>
  <div class="backtest-view">
    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>执行回测</template>
          <el-form label-width="80px">
            <el-form-item label="信号ID">
              <el-input-number v-model="signal_id":min="1" style="width:100%" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="evaluating" :disabled="!signalId" @click="runEvaluate">
                评估信号
              </el-button>
              <el-button :loading="batching" @click="runBatch" style="margin-left:8px">
                批量评估
              </el-button>
            </el-form-item>
          </el-form>
          <el-alert v-if="evalResult" :title="evalTitle" :type="evalResult.outcome === 'win' ? 'success' : evalResult.outcome === 'loss' ? 'error' : 'info'" show-icon :closable="false" style="margin-top:12px">
            <div>收益率: {{ formatPct(evalResult.stockReturnPct) }}</div>
            <div>最大回撤: {{ formatPct(evalResult.maxDrawdown) }}</div>
            <div>方向正确: {{ evalResult.directionCorrect ? '是' : '否' }}</div>
          </el-alert>
        </el-card>
      </el-col>
      <el-col :span="16">
        <el-row :gutter="16">
          <el-col :span="6" v-for="card in perfCards" :key="card.label">
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
          <span>回测列表</span>
          <div style="display:flex;gap:8px;align-items:center">
            <el-select v-model="listOutcomeFilter" placeholder="筛选结果" clearable style="width:120px" @change="loadList">
              <el-option label="命中(win)" value="win" />
              <el-option label="未中(loss)" value="loss" />
              <el-option label="中性" value="neutral" />
            </el-select>
            <el-input v-model="listCodeFilter" placeholder="按代码筛选" clearable style="width:160px" @clear="loadList" @keyup.enter="loadList">
              <template #append>
                <el-button @click="loadList">查询</el-button>
              </template>
            </el-input>
          </div>
        </div>
      </template>
      <el-table :data="backtestList" stripe style="width:100%" v-loading="listLoading">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column label="代码" width="100">
          <template #default="{ row }">{{ row.stockCode || row.code || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80">
          <template #default="{ row }">{{ actionLabel(row.decisionAction || row.action) }}</template>
        </el-table-column>
        <el-table-column label="入场价" width="90">
          <template #default="{ row }">{{ row.simulatedEntry || row.startPrice || '-' }}</template>
        </el-table-column>
        <el-table-column label="收益%" width="100">
          <template #default="{ row }">
            <span :class="pnlClass(row.stockReturnPct || row.returnPct)">
              {{ formatPct(row.stockReturnPct || row.returnPct) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="最大回撤" width="100">
          <template #default="{ row }">
            <span style="color:#f56c6c">{{ formatPct(row.maxDrawdown) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="结果" width="80">
          <template #default="{ row }">
            <el-tag :type="outcomeTag(row.outcome)" size="small">{{ outcomeLabel(row.outcome) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="方向" width="80">
          <template #default="{ row }">
            <el-tag :type="row.directionCorrect ? 'success' : 'danger'" size="small">
              {{ row.directionCorrect ? '正确' : '错误' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="信号日期" width="110">
          <template #default="{ row }">{{ row.signalDate || '-' }}</template>
        </el-table-column>
        <el-table-column label="创建时间" width="160">
          <template #default="{ row }">{{ row.createTime || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="viewDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!backtestList.length && !listLoading" description="暂无回测记录，请先产生决策信号再执行回测" />
    </el-card>

    <el-dialog v-model="detailVisible" title="回测详情" width="700px">
      <el-descriptions :column="2" border v-if="detailData">
        <el-descriptions-item label="股票代码">{{ detailData.stockCode || '-' }}</el-descriptions-item>
        <el-descriptions-item label="决策操作">{{ actionLabel(detailData.decisionAction) }}</el-descriptions-item>
        <el-descriptions-item label="信号日期">{{ detailData.signalDate || '-' }}</el-descriptions-item>
        <el-descriptions-item label="评估窗口">{{ detailData.evalWindowDays || '-' }}天</el-descriptions-item>
        <el-descriptions-item label="起始价">{{ detailData.startPrice || '-' }}</el-descriptions-item>
        <el-descriptions-item label="终价">{{ detailData.endClose || '-' }}</el-descriptions-item>
        <el-descriptions-item label="最高价">{{ detailData.maxHigh || '-' }}</el-descriptions-item>
        <el-descriptions-item label="最低价">{{ detailData.minLow || '-' }}</el-descriptions-item>
        <el-descriptions-item label="收益率">
          <span :class="pnlClass(detailData.stockReturnPct)">{{ formatPct(detailData.stockReturnPct) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="最大回撤">
          <span style="color:#f56c6c">{{ formatPct(detailData.maxDrawdown) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="预期方向">{{ detailData.directionExpected || '-' }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="outcomeTag(detailData.outcome)">{{ outcomeLabel(detailData.outcome) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="方向正确">
          <el-tag :type="detailData.directionCorrect ? 'success' : 'danger'">{{ detailData.directionCorrect ? '是' : '否' }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="触发止损">{{ detailData.hitStopLoss ? '是' : '否' }}</el-descriptions-item>
        <el-descriptions-item label="触发止盈">{{ detailData.hitTakeProfit ? '是' : '否' }}</el-descriptions-item>
        <el-descriptions-item label="止损价">{{ detailData.stopLossPrice || '-' }}</el-descriptions-item>
        <el-descriptions-item label="止盈价">{{ detailData.takeProfitPrice || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { backtestApi } from '@/api/backtest'

const signalId = ref(0)
const evaluating = ref(false)
const batching = ref(false)
const evalResult = ref<Record<string, any> | null>(null)
const backtestList = ref<any[]>([])
const listLoading = ref(false)
const listCodeFilter = ref('')
const listOutcomeFilter = ref('')
const summaryData = ref<Record<string, any>>({})
const detailVisible = ref(false)
const detailData = ref<Record<string, any> | null>(null)

const evalTitle = computed(() => {
  if (!evalResult.value) return ''
  const r = evalResult.value
  return `${r.stockCode || ''} - ${actionLabel(r.decisionAction || r.directionExpected)}`
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

function actionLabel(action: string) {
  const map: Record<string, string> = { buy: '买入', sell: '卖出', hold: '持有', add: '加仓', reduce: '减仓', watch: '关注', avoid: '回避', up: '看涨', down: '看跌' }
  return map[action] || action || '-'
}

function pnlClass(val: number | undefined) {
  const v = Number(val || 0)
  return v > 0 ? 'pnl-up' : v < 0 ? 'pnl-down' : ''
}

function formatPct(val: number | undefined) {
  const v = Number(val || 0)
  return (v >= 0 ? '+' : '') + v.toFixed(2) + '%'
}

function outcomeTag(outcome: string) {
  return outcome === 'win' ? 'success' : outcome === 'loss' ? 'danger' : 'info'
}

function outcomeLabel(outcome: string) {
  const map: Record<string, string> = { win: '命中', loss: '未中', neutral: '中性' }
  return map[outcome] || outcome || '-'
}

async function runEvaluate() {
  if (!signalId.value) {
    ElMessage.warning('请输入信号ID')
    return
  }
  evaluating.value = true
  try {
    const res: any = await backtestApi.evaluate(signalId.value)
    evalResult.value = res.data || null
    ElMessage.success('评估完成')
    loadList()
    loadSummary()
  } catch (e: any) {
    ElMessage.error(e?.message || '评估失败')
  } finally {
    evaluating.value = false
  }
}

async function runBatch() {
  batching.value = true
  try {
    const res: any = await backtestApi.evaluateBatch(50)
    const d = res.data || {}
    ElMessage.success(`批量评估完成: ${d.evaluatedCount || 0}条成功, ${d.errors?.length || 0}条失败`)
    loadList()
    loadSummary()
  } catch (e: any) {
    ElMessage.error(e?.message || '批量评估失败')
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
  finally {
    listLoading.value = false
  }
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
  loadList()
  loadSummary()
})
</script>

<style scoped lang="scss">
.perf-card {
  text-align: center;
  margin-bottom: 16px;
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
