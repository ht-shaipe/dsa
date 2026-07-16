<template>
  <div class="portfolio-view">
    <el-card shadow="hover" class="summary-bar" style="margin-bottom: 20px">
      <div class="summary-row">
        <div class="summary-item">
          <span class="summary-label">总市值</span>
          <span class="summary-value">¥{{ formatNum(summary.totalValue, 2) }}</span>
        </div>
        <el-divider direction="vertical" />
        <div class="summary-item">
          <span class="summary-label">总盈亏</span>
          <span :class="['summary-value', pnlClass(summary.totalPnl)]">{{ pnlText(summary.totalPnl) }}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">收益率</span>
          <span :class="['summary-value', pnlClass(summary.totalPnl)]">
            {{ (Number(summary.totalPnl || 0) >= 0 ? '+' : '') }}{{ formatNum(summary.totalPnlPct || pnlPercent, 2) }}%
          </span>
        </div>
        <el-divider direction="vertical" />
        <div class="summary-item">
          <span class="summary-label">持仓数</span>
          <span class="summary-value">{{ summary.positionCount || 0 }}</span>
        </div>
      </div>
    </el-card>

    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <div style="display:flex;justify-content:space-between;align-items:center">
              <span>持仓明细</span>
              <div>
                <el-button type="success" @click="openTradeDialog('buy')">买入</el-button>
                <el-button type="danger" @click="openTradeDialog('sell')">卖出</el-button>
                <el-button type="primary" @click="openBatchDialog">批量录入</el-button>
              </div>
            </div>
          </template>
          <PositionCardList
            :positions="positions"
            :columns="4"
            empty-text="暂无持仓，点击「买入」添加"
            @click="onPositionClick"
          />
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>交易记录</template>
      <el-table :data="trades" stripe style="width:100%">
        <el-table-column prop="stockCode" label="代码" width="100" />
        <el-table-column prop="stockName" label="名称" width="120" />
        <el-table-column label="方向" width="80">
          <template #default="{ row }">
            <el-tag :type="(row.side || row.direction) === 'buy' ? 'success' : 'danger'" >
              {{ (row.side || row.direction) === 'buy' ? '买入' : '卖出' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="price" label="价格" width="100">
          <template #default="{ row }">{{ Number(row.price || 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" width="100" />
        <el-table-column prop="commission" label="佣金" width="100">
          <template #default="{ row }">{{ Number(row.commission || 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="tradeTime" label="时间" width="180">
          <template #default="{ row }">{{ row.tradeDate || row.tradeTime || row.createdAt || row.createdTime || '-' }}</template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
      </el-table>
      <el-empty v-if="!trades.length" description="暂无交易记录" />
    </el-card>

    <el-dialog v-model="tradeDialogVisible" :title="tradeDirection === 'buy' ? '买入股票' : '卖出股票'" width="500px">
      <el-form :model="tradeForm" label-width="80px">
        <el-form-item label="股票代码">
          <StockAutocomplete v-model="tradeForm.code" @select="onTradeStockSelect" />
        </el-form-item>
        <el-form-item label="股票名称">
          <el-input v-model="tradeForm.name" disabled />
        </el-form-item>
        <el-form-item label="交易时间">
          <el-date-picker v-model="tradeForm.tradeDate" type="datetime" placeholder="实际交易时间" style="width:100%" value-format="YYYY-MM-DD HH:mm:ss" />
        </el-form-item>
        <el-form-item label="价格">
          <el-input-number v-model="tradeForm.price" :precision="2" :min="0" style="width:100%" />
        </el-form-item>
        <el-form-item label="数量">
          <el-input-number v-model="tradeForm.quantity" :min="1" :step="100" style="width:100%" />
        </el-form-item>
        <el-form-item label="佣金">
          <el-input-number v-model="tradeForm.commission" :precision="2" :min="0" style="width:100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="tradeForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tradeDialogVisible = false">取消</el-button>
        <el-button :type="tradeDirection === 'buy' ? 'success' : 'danger'" :loading="tradeSubmitting" @click="submitTrade">
          确认{{ tradeDirection === 'buy' ? '买入' : '卖出' }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="batchDialogVisible" title="批量录入交易" width="820px" @close="resetBatchForm">
      <el-form label-width="90px" style="margin-bottom:12px">
        <el-form-item label="股票代码">
          <StockAutocomplete v-model="batchForm.code" @select="onBatchStockSelect" style="width:220px" />
        </el-form-item>
        <el-form-item label="股票名称">
          <el-input v-model="batchForm.name" disabled style="width:220px" />
        </el-form-item>
      </el-form>

      <el-table :data="batchForm.rows" border  style="width:100%">
        <el-table-column label="方向" width="100">
          <template #default="{ row }">
            <el-select v-model="row.direction" >
              <el-option label="买入" value="buy" />
              <el-option label="卖出" value="sell" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="日期" width="158">
          <template #default="{ row }">
            <el-date-picker v-model="row.tradeDate" type="date"  style="width:100%" value-format="YYYY-MM-DD" placeholder="交易日期" />
          </template>
        </el-table-column>
        <el-table-column label="价格" width="150">
          <template #default="{ row }">
            <el-input-number v-model="row.price" :precision="3" :min="0"  style="width:100%" />
          </template>
        </el-table-column>
        <el-table-column label="数量" width="170">
          <template #default="{ row }">
            <el-input-number v-model="row.quantity" :min="1" :step="100"  style="width:100%" />
          </template>
        </el-table-column>
        <el-table-column label="佣金" width="150">
          <template #default="{ row }">
            <el-input-number v-model="row.commission" :precision="3" :min="0"  style="width:100%" />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="60" fixed="right">
          <template #default="{ $index }">
            <el-button link type="danger"  @click="removeBatchRow($index)">删</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div style="margin-top:8px;display:flex;gap:8px">
        <el-button  @click="addBatchRow('buy')">+ 买入行</el-button>
        <el-button  @click="addBatchRow('sell')">+ 卖出行</el-button>
      </div>

      <template #footer>
        <el-button @click="batchDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="batchSubmitting" :disabled="!batchForm.code || !batchForm.rows.length" @click="submitBatch">
          提交 {{ batchForm.rows.length }} 笔交易
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { portfolioApi } from '@/api/portfolio'
import { useTradingInterval } from '@/composables/useTradingInterval'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import PositionCardList from '@/components/common/PositionCardList.vue'
import { formatNum, pnlText, pnlClass } from '@/utils/format'

const summary = ref<Record<string, any>>({})
const positions = ref<any[]>([])
const trades = ref<any[]>([])
const accounts = ref<any[]>([])

const tradeDialogVisible = ref(false)
const tradeDirection = ref<'buy' | 'sell'>('buy')
const tradeSubmitting = ref(false)
const tradeForm = ref({
  code: '',
  name: '',
  tradeDate: '',
  price: 0,
  quantity: 100,
  commission: 0,
  remark: '',
})

const batchDialogVisible = ref(false)
const batchSubmitting = ref(false)
const batchForm = ref({
  code: '',
  name: '',
  rows: [] as { direction: string; tradeDate: string; price: number; quantity: number; commission: number }[],
})

const tradingTimer = useTradingInterval(() => {
  loadData()
}, 10000)

const pnlPercent = computed(() => {
  const tv = summary.value.totalValue || 0
  const tp = summary.value.totalPnl || 0
  const tc = summary.value.totalCost || 0
  if (tc > 0) return (tp / tc) * 100
  if (!tv) return 0
  return (tp / (tv - tp)) * 100
})

function onPositionClick(row: any) {
  tradeForm.value.code = row.stockCode || row.code || ''
  tradeForm.value.name = row.stockName || row.name || ''
}

function openTradeDialog(dir: 'buy' | 'sell') {
  tradeDirection.value = dir
  tradeForm.value = { code: '', name: '', tradeDate: '', price: 0, quantity: 100, commission: 0, remark: '' }
  tradeDialogVisible.value = true
}

function onTradeStockSelect(code: string, name: string) {
  tradeForm.value.code = code
  tradeForm.value.name = name
}

async function submitTrade() {
  if (!tradeForm.value.code) {
    ElMessage.warning('请选择股票')
    return
  }
  tradeSubmitting.value = true
  try {
    const accountId = accounts.value[0]?.id || 1
    const { code, price, quantity, name, commission, remark, tradeDate } = tradeForm.value
    const tradeParams: Record<string, any> = { accountId, code, price, quantity, name, commission, remark }
    if (tradeDate) tradeParams.tradeDate = tradeDate
    if (tradeDirection.value === 'buy') {
      await portfolioApi.add(tradeParams as any)
      ElMessage.success('买入成功')
    } else {
      await portfolioApi.remove(tradeParams as any)
      ElMessage.success('卖出成功')
    }
    tradeDialogVisible.value = false
    loadData()
  } catch {
    ElMessage.error(tradeDirection.value === 'buy' ? '买入失败' : '卖出失败')
  } finally {
    tradeSubmitting.value = false
  }
}

function openBatchDialog() {
  batchForm.value = { code: '', name: '', rows: [] }
  addBatchRow('buy')
  addBatchRow('buy')
  addBatchRow('buy')
  batchDialogVisible.value = true
}

function resetBatchForm() {
  batchForm.value = { code: '', name: '', rows: [] }
}

function addBatchRow(direction: string) {
  batchForm.value.rows.push({ direction, tradeDate: '', price: 0, quantity: 100, commission: 0 })
}

function removeBatchRow(index: number) {
  batchForm.value.rows.splice(index, 1)
}

function onBatchStockSelect(code: string, name: string) {
  batchForm.value.code = code
  batchForm.value.name = name
}

async function submitBatch() {
  if (!batchForm.value.code) {
    ElMessage.warning('请选择股票')
    return
  }
  const validRows = batchForm.value.rows.filter(r => r.price > 0 && r.quantity > 0)
  if (!validRows.length) {
    ElMessage.warning('请至少填写一行有效的交易（价格和数量大于0）')
    return
  }
  batchSubmitting.value = true
  const accountId = accounts.value[0]?.id || 1
  let successCount = 0
  let failCount = 0
  try {
    for (const row of validRows) {
      const params: Record<string, any> = {
        accountId,
        code: batchForm.value.code,
        name: batchForm.value.name,
        price: row.price,
        quantity: row.quantity,
        commission: row.commission,
        remark: '',
      }
      if (row.tradeDate) params.tradeDate = row.tradeDate
      try {
        if (row.direction === 'buy') {
          await portfolioApi.add(params as any)
        } else {
          await portfolioApi.remove(params as any)
        }
        successCount++
      } catch {
        failCount++
      }
    }
    if (failCount === 0) {
      ElMessage.success(`全部 ${successCount} 笔交易录入成功`)
    } else {
      ElMessage.warning(`${successCount} 笔成功，${failCount} 笔失败`)
    }
    batchDialogVisible.value = false
    loadData()
  } finally {
    batchSubmitting.value = false
  }
}

async function loadData() {
  try {
    const [summaryRes, positionsRes, tradesRes, accountsRes] = await Promise.all([
      portfolioApi.summary(),
      portfolioApi.positions(),
      portfolioApi.trades(),
      portfolioApi.accounts(),
    ])
    summary.value = (summaryRes as any) || {}
    positions.value = (positionsRes as any) || []
    trades.value = (tradesRes as any) || []
    accounts.value = (accountsRes as any) || []
  } catch { /* ignore */ }
}

onMounted(() => {
  tradingTimer.start()
})

onUnmounted(() => {
  tradingTimer.stop()
})
</script>

<style scoped lang="scss">
.summary-bar {
  :deep(.el-card__body) { padding: 0; }
}
.summary-row {
  display: flex;
  align-items: center;
  padding: 14px 20px;
}
.summary-item {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-left: 6px;
}
.summary-label {
  font-size: 13px;
  color: var(--el-text-color-secondary);
}
.summary-value {
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.pnl-up {
  color: #f56c6c;
}
.pnl-down {
  color: #67c23a;
}
:deep(.el-divider--vertical) {
  height: 28px;
  margin: 0 20px;
}
</style>
