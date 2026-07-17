<template>
  <div class="portfolio-view">
    <el-card shadow="hover" class="summary-bar" style="margin-bottom: 20px">
      <div class="summary-row">
        <div class="summary-item">
          <span class="summary-label">总资产</span>
          <span class="summary-value">¥{{ formatNum(summary.totalEquity || summary.totalValue, 2) }}</span>
        </div>
        <el-divider direction="vertical" />
        <div class="summary-item">
          <span class="summary-label">总市值</span>
          <span class="summary-value">¥{{ formatNum(summary.totalValue, 2) }}</span>
        </div>
        <el-divider direction="vertical" />
        <div class="summary-item">
          <span class="summary-label">现金</span>
          <span class="summary-value">¥{{ formatNum(summary.cashBalance, 2) }}</span>
        </div>
        <el-divider direction="vertical" />
        <div class="summary-item">
          <span class="summary-label">已实现盈亏</span>
          <span :class="['summary-value', pnlClass(summary.realizedPnl)]">{{ pnlText(summary.realizedPnl) }}</span>
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
                <el-button type="warning" @click="ocrDialogVisible = true">截图导入</el-button>
                <el-button @click="rebuildPositions" :loading="rebuilding">重建持仓</el-button>
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
        <el-table-column prop="remark" label="备注" min-width="120" show-overflow-tooltip />
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEditTradeDialog(row)">编辑</el-button>
            <el-button type="danger" link size="small" @click="handleDeleteTrade(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!trades.length" description="暂无交易记录" />
    </el-card>

    <el-dialog v-model="editTradeDialogVisible" title="编辑交易" width="500px">
      <el-form :model="editTradeForm" label-width="80px">
        <el-form-item label="股票代码">
          <el-input v-model="editTradeForm.code" />
        </el-form-item>
        <el-form-item label="股票名称">
          <el-input v-model="editTradeForm.name" />
        </el-form-item>
        <el-form-item label="方向">
          <el-select v-model="editTradeForm.direction" style="width:100%">
            <el-option label="买入" value="buy" />
            <el-option label="卖出" value="sell" />
          </el-select>
        </el-form-item>
        <el-form-item label="交易时间">
          <el-date-picker v-model="editTradeForm.tradeDate" type="datetime" placeholder="交易时间" style="width:100%" value-format="YYYY-MM-DD HH:mm:ss" />
        </el-form-item>
        <el-form-item label="价格">
          <el-input-number v-model="editTradeForm.price" :precision="2" :min="0" style="width:100%" />
        </el-form-item>
        <el-form-item label="数量">
          <el-input-number v-model="editTradeForm.quantity" :min="1" :step="100" style="width:100%" />
        </el-form-item>
        <el-form-item label="佣金">
          <el-input-number v-model="editTradeForm.commission" :precision="2" :min="0" style="width:100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="editTradeForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editTradeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="editTradeSubmitting" @click="submitEditTrade">保存</el-button>
      </template>
    </el-dialog>

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

    <el-dialog v-model="ocrDialogVisible" title="截图识别导入交易" width="600px" @close="resetOcrForm">
      <div class="ocr-upload-area">
        <div v-if="!ocrPreviewUrl" class="ocr-drop-zone" @click="triggerOcrFileInput" @paste="onOcrPaste" @dragover.prevent @drop.prevent="onOcrDrop">
          <el-icon :size="40" style="color:var(--el-text-color-placeholder)"><UploadFilled /></el-icon>
          <p>点击选择图片 / 拖拽到此处 / Ctrl+V 粘贴截图</p>
          <p style="font-size:12px;color:var(--el-text-color-placeholder)">支持交易软件截图，自动识别股票代码、价格、数量等信息</p>
        </div>
        <div v-else class="ocr-preview">
          <img :src="ocrPreviewUrl" style="max-width:100%;max-height:300px;border-radius:8px" />
          <el-button link type="danger" @click="resetOcrForm" style="margin-top:8px">清除重选</el-button>
        </div>
        <input ref="ocrFileRef" type="file" accept="image/*" style="display:none" @change="onOcrFileChange" />
      </div>
      <div v-if="ocrResult" style="margin-top:12px">
        <el-alert
          :title="`识别完成：共 ${ocrResult.total} 笔，成功 ${ocrResult.success} 笔，失败 ${ocrResult.failed} 笔`"
          :type="ocrResult.failed > 0 ? 'warning' : 'success'"
          show-icon
          :closable="false"
        />
        <div v-if="ocrResult.errors && ocrResult.errors.length" style="margin-top:8px">
          <div v-for="(err, i) in ocrResult.errors" :key="i" style="color:var(--el-color-danger);font-size:12px">
            第{{ err.index + 1 }}笔: {{ err.error }}
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="ocrDialogVisible = false">关闭</el-button>
        <el-button type="primary" :loading="ocrSubmitting" :disabled="!ocrBase64" @click="submitOcrImport">
          识别并导入
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { UploadFilled } from '@element-plus/icons-vue'
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
const rebuilding = ref(false)
const ocrDialogVisible = ref(false)
const ocrSubmitting = ref(false)
const ocrBase64 = ref('')
const ocrPreviewUrl = ref('')
const ocrResult = ref<any>(null)
const ocrFileRef = ref<HTMLInputElement | null>(null)
const batchForm = ref({
  code: '',
  name: '',
  rows: [] as { direction: string; tradeDate: string; price: number; quantity: number; commission: number }[],
})

const editTradeDialogVisible = ref(false)
const editTradeSubmitting = ref(false)
const editTradeForm = ref({
  id: 0,
  code: '',
  name: '',
  direction: 'buy' as 'buy' | 'sell',
  tradeDate: '',
  price: 0,
  quantity: 100,
  commission: 0,
  remark: '',
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

async function rebuildPositions() {
  const accountId = accounts.value[0]?.id || 1
  rebuilding.value = true
  try {
    const res: any = await portfolioApi.rebuild(accountId)
    const count = res?.rebuiltPositions ?? 0
    ElMessage.success(`持仓重建完成: ${count} 个持仓已从交易记录重新计算`)
    loadData()
  } catch {
    ElMessage.error('重建持仓失败')
  } finally {
    rebuilding.value = false
  }
}

function triggerOcrFileInput() {
  ocrFileRef.value?.click()
}

function resetOcrForm() {
  ocrBase64.value = ''
  ocrPreviewUrl.value = ''
  ocrResult.value = null
}

function onOcrFileChange(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  readOcrFile(file)
}

function onOcrDrop(e: DragEvent) {
  const file = e.dataTransfer?.files?.[0]
  if (!file || !file.type.startsWith('image/')) return
  readOcrFile(file)
}

function onOcrPaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items
  if (!items) return
  for (const item of items) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile()
      if (file) { readOcrFile(file); break }
    }
  }
}

function readOcrFile(file: File) {
  const reader = new FileReader()
  reader.onload = (ev) => {
    const dataUrl = ev.target?.result as string
    ocrPreviewUrl.value = dataUrl
    ocrBase64.value = dataUrl.split(',')[1] || dataUrl
  }
  reader.readAsDataURL(file)
}

async function submitOcrImport() {
  if (!ocrBase64.value) {
    ElMessage.warning('请先选择或粘贴截图')
    return
  }
  const accountId = accounts.value[0]?.id || 1
  ocrSubmitting.value = true
  ocrResult.value = null
  try {
    const res: any = await portfolioApi.ocrImport({ accountId, image: ocrBase64.value })
    ocrResult.value = res
    if (res.success > 0) {
      ElMessage.success(`截图识别导入成功 ${res.success} 笔交易`)
      loadData()
    }
    if (res.failed > 0) {
      ElMessage.warning(`${res.failed} 笔交易识别/导入失败，请检查详情`)
    }
  } catch {
    ElMessage.error('截图识别失败，请确认LLM视觉模型已配置')
  } finally {
    ocrSubmitting.value = false
  }
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

function openEditTradeDialog(row: any) {
  editTradeForm.value = {
    id: row.id,
    code: row.stockCode || row.code || '',
    name: row.stockName || row.name || '',
    direction: (row.side || row.direction) === 'sell' ? 'sell' : 'buy',
    tradeDate: row.tradeDate || row.tradeTime || '',
    price: Number(row.price || 0),
    quantity: Number(row.quantity || 0),
    commission: Number(row.commission || 0),
    remark: row.remark || '',
  }
  editTradeDialogVisible.value = true
}

async function submitEditTrade() {
  if (!editTradeForm.value.code) {
    ElMessage.warning('股票代码不能为空')
    return
  }
  editTradeSubmitting.value = true
  try {
    const { id, code, name, direction, tradeDate, price, quantity, commission, remark } = editTradeForm.value
    await portfolioApi.editTrade({ id, code, name, direction, price, quantity, commission, remark, tradeDate: tradeDate || undefined })
    ElMessage.success('交易修改成功，持仓已重建')
    editTradeDialogVisible.value = false
    loadData()
  } catch {
    ElMessage.error('修改交易失败')
  } finally {
    editTradeSubmitting.value = false
  }
}

async function handleDeleteTrade(row: any) {
  const dir = (row.side || row.direction) === 'buy' ? '买入' : '卖出'
  const code = row.stockCode || row.code || ''
  try {
    await ElMessageBox.confirm(
      `确认删除 ${code} ${dir} ${row.quantity}股 @${Number(row.price || 0).toFixed(2)} 的交易记录？持仓将自动重建。`,
      '删除交易',
      { confirmButtonText: '确认删除', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }
  try {
    await portfolioApi.deleteTrade(row.id)
    ElMessage.success('交易已删除，持仓已重建')
    loadData()
  } catch {
    ElMessage.error('删除交易失败')
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
.ocr-upload-area {
  margin-bottom: 12px;
}
.ocr-drop-zone {
  border: 2px dashed var(--el-border-color);
  border-radius: 8px;
  padding: 40px 20px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s;
  &:hover { border-color: var(--el-color-primary); }
  p { margin: 8px 0 0; color: var(--el-text-color-regular); }
}
.ocr-preview {
  text-align: center;
}
</style>
