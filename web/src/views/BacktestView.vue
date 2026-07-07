<template>
  <div class="backtest-view">
    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>执行回测</template>
          <el-form :model="runForm" label-width="80px">
            <el-form-item label="股票">
              <StockAutocomplete @select="onSelectStock" />
            </el-form-item>
            <el-form-item label="分析ID">
              <el-input-number v-model="runForm.analysisId" :min="1" style="width:100%" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="running" :disabled="!runForm.analysisId" @click="runBacktest">
                执行回测
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
      <el-col :span="16">
        <el-row :gutter="16">
          <el-col :span="8" v-for="card in perfCards" :key="card.label">
            <el-card shadow="hover" class="perf-card">
              <el-statistic :title="card.label" :value="card.value" :precision="card.precision || 2" :suffix="card.suffix || ''" />
            </el-card>
          </el-col>
        </el-row>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>回测列表</span>
          <el-input v-model="listCodeFilter" placeholder="按代码筛选" clearable style="width:200px" @clear="loadList" @keyup.enter="loadList">
            <template #append>
              <el-button @click="loadList">查询</el-button>
            </template>
          </el-input>
        </div>
      </template>
      <el-table :data="backtestList" stripe style="width:100%" v-loading="listLoading">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="code" label="代码" width="100" />
        <el-table-column prop="name" label="名称" width="120" />
        <el-table-column prop="strategy" label="策略" width="120" />
        <el-table-column label="总收益" width="120">
          <template #default="{ row }">
            <span :class="pnlClass(row.totalReturn || row.total_return)">
              {{ formatPercent(row.totalReturn || row.total_return) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="胜率" width="100">
          <template #default="{ row }">{{ formatPercent(row.winRate || row.win_rate) }}</template>
        </el-table-column>
        <el-table-column label="最大回撤" width="120">
          <template #default="{ row }">
            <span style="color:#f56c6c">{{ formatPercent(row.maxDrawdown || row.max_drawdown) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="结果" width="80">
          <template #default="{ row }">
            <el-tag :type="outcomeTag(row.outcome)" size="small">
              {{ outcomeLabel(row.outcome) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ row.createdAt || row.created_at || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="viewDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!backtestList.length && !listLoading" description="暂无回测记录" />
    </el-card>

    <el-dialog v-model="detailVisible" title="回测详情" width="700px">
      <el-descriptions :column="2" border v-if="detailData">
        <el-descriptions-item label="代码">{{ detailData.code }}</el-descriptions-item>
        <el-descriptions-item label="名称">{{ detailData.name }}</el-descriptions-item>
        <el-descriptions-item label="策略">{{ detailData.strategy }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="outcomeTag(detailData.outcome)">{{ outcomeLabel(detailData.outcome) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="总收益">{{ formatPercent(detailData.totalReturn || detailData.total_return) }}</el-descriptions-item>
        <el-descriptions-item label="胜率">{{ formatPercent(detailData.winRate || detailData.win_rate) }}</el-descriptions-item>
        <el-descriptions-item label="最大回撤">{{ formatPercent(detailData.maxDrawdown || detailData.max_drawdown) }}</el-descriptions-item>
        <el-descriptions-item label="夏普比率">{{ detailData.sharpeRatio || detailData.sharpe_ratio || '-' }}</el-descriptions-item>
      </el-descriptions>
      <div v-if="detailData?.trades?.length" style="margin-top:16px">
        <el-divider content-position="left">交易明细</el-divider>
        <el-table :data="detailData.trades" stripe size="small" max-height="300">
          <el-table-column prop="code" label="代码" width="80" />
          <el-table-column label="方向" width="60">
            <template #default="{ row }">{{ row.side === 'buy' ? '买' : '卖' }}</template>
          </el-table-column>
          <el-table-column prop="price" label="价格" width="80" />
          <el-table-column prop="quantity" label="数量" width="60" />
          <el-table-column prop="date" label="日期" width="100" />
        </el-table>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { backtestApi } from '@/api/backtest'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'

const runForm = ref({ analysisId: 0, code: '', name: '' })
const running = ref(false)
const backtestList = ref<any[]>([])
const listLoading = ref(false)
const listCodeFilter = ref('')
const performance = ref<Record<string, any>>({})
const detailVisible = ref(false)
const detailData = ref<Record<string, any> | null>(null)

const perfCards = computed(() => [
  { label: '总收益', value: (performance.value.totalReturn || performance.value.total_return || 0) * 100, precision: 2, suffix: '%' },
  { label: '胜率', value: (performance.value.winRate || performance.value.win_rate || 0) * 100, precision: 1, suffix: '%' },
  { label: '最大回撤', value: (performance.value.maxDrawdown || performance.value.max_drawdown || 0) * 100, precision: 2, suffix: '%' },
])

function onSelectStock(code: string, name: string) {
  runForm.value.code = code
  runForm.value.name = name
}

function pnlClass(val: number | undefined) {
  const v = Number(val || 0)
  return v > 0 ? 'pnl-up' : v < 0 ? 'pnl-down' : ''
}

function formatPercent(val: number | undefined) {
  const v = Number(val || 0)
  const pct = Math.abs(v) > 1 ? v : v * 100
  return (pct >= 0 ? '+' : '') + pct.toFixed(2) + '%'
}

function outcomeTag(outcome: string) {
  return outcome === 'hit' ? 'success' : 'danger'
}

function outcomeLabel(outcome: string) {
  return outcome === 'hit' ? '命中' : outcome === 'miss' ? '未中' : outcome || '-'
}

async function runBacktest() {
  if (!runForm.value.analysisId) {
    ElMessage.warning('请输入分析ID')
    return
  }
  running.value = true
  try {
    await backtestApi.run(runForm.value.analysisId)
    ElMessage.success('回测已启动')
    loadList()
    loadPerformance()
  } catch {
    ElMessage.error('回测执行失败')
  } finally {
    running.value = false
  }
}

async function loadList() {
  listLoading.value = true
  try {
    const params: Record<string, any> = {}
    if (listCodeFilter.value) params.code = listCodeFilter.value
    const res: any = await backtestApi.list(params)
    backtestList.value = res.data || []
  } catch { /* ignore */ }
  finally {
    listLoading.value = false
  }
}

async function loadPerformance() {
  try {
    const res: any = await backtestApi.performance(listCodeFilter.value || undefined)
    performance.value = res.data || {}
  } catch { /* ignore */ }
}

async function viewDetail(row: Record<string, any>) {
  try {
    const res: any = await backtestApi.detail(row.id)
    detailData.value = res.data || row
    detailVisible.value = true
  } catch {
    detailData.value = row
    detailVisible.value = true
  }
}

onMounted(() => {
  loadList()
  loadPerformance()
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
