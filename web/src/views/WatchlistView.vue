<template>
  <div class="watchlist-view">
    <el-card shadow="hover">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>自选股管理</span>
          <div style="display:flex;gap:8px;align-items:center">
            <el-button :icon="Refresh" circle :loading="refreshing" @click="handleRefreshQuotes" title="刷新行情" />
            <StockAutocomplete
              v-model="searchText"
              @select="(code: string, name: string) => { addStock(code, name); searchText = '' }"
              placeholder="搜索添加股票..."
              style="width:220px"
            />
          </div>
        </div>
      </template>

      <el-table :data="stocks" stripe style="width:100%" v-loading="loading">
        <el-table-column label="代码" width="100">
          <template #default="{ row }">
            <span style="font-weight:500">{{ row.stockCode || row.code }}</span>
          </template>
        </el-table-column>
        <el-table-column label="名称" width="120">
          <template #default="{ row }">{{ row.stockName || row.name || '-' }}</template>
        </el-table-column>
        <el-table-column label="现价" >
          <template #default="{ row }">
            <span v-if="row.close != null" :class="Number(row.changePercent) >= 0 ? 'pnl-up' : 'pnl-down'">{{ Number(row.close).toFixed(2) }}</span>
            <span v-else-if="row.price != null" :class="Number(row.changePercent) >= 0 ? 'pnl-up' : 'pnl-down'">{{ Number(row.price).toFixed(2) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="涨跌幅" >
          <template #default="{ row }">
            <span v-if="row.changePercent != null" :class="Number(row.changePercent) >= 0 ? 'pnl-up' : 'pnl-down'">
              {{ Number(row.changePercent) >= 0 ? '+' : '' }}{{ Number(row.changePercent).toFixed(2) }}%
            </span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="分组" width="100">
          <template #default="{ row }">
            <el-tag size="small" type="info">{{ row.groupName || 'default' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="添加时间" width="168">
          <template #default="{ row }">{{ formatDateTime(row.createTime) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="editStock(row)">编辑</el-button>
            <el-button link type="primary" @click="analyzeStock(row)">分析</el-button>
            <el-popconfirm title="确定移除?" @confirm="removeStock(row)">
              <template #reference>
                <el-button link type="danger">移除</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!stocks.length && !loading" description="暂无自选股，搜索股票添加" />
    </el-card>

    <el-dialog v-model="editVisible" title="编辑自选股" width="440px">
      <el-form :model="editForm" label-width="80px" v-if="editForm">
        <el-form-item label="股票代码">
          <el-input :model-value="editForm.stockCode" disabled />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="editForm.stockName" />
        </el-form-item>
        <el-form-item label="分组">
          <el-input v-model="editForm.groupName" placeholder="default" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="editForm.sortOrder" :min="0" :max="999" style="width:100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="editForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="saveEdit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import { stockApi } from '@/api/stock'
import { useTradingInterval } from '@/composables/useTradingInterval'
import { formatDateTime } from '@/utils/format'

const router = useRouter()
const stocks = ref<any[]>([])
const loading = ref(false)
const saving = ref(false)
const refreshing = ref(false)
const editVisible = ref(false)
const editForm = ref<Record<string, any> | null>(null)
const searchText = ref('')

const quoteTimer = useTradingInterval(refreshQuotes, 10000)

async function loadList() {
  loading.value = true
  try {
    const res: any = await stockApi.watchlist()
    stocks.value = Array.isArray(res) ? res : []
    if (stocks.value.length > 0 && stocks.value[0].id === 0) {
      const codes = stocks.value.map((s: any) => ({ code: s.stockCode || s.code, name: s.stockName || s.name || '' }))
      await stockApi.watchlistSync(codes)
      const res2: any = await stockApi.watchlist()
      stocks.value = Array.isArray(res2) ? res2 : []
    }
  } catch {
    stocks.value = []
  } finally {
    loading.value = false
  }
  quoteTimer.start()
}

async function refreshQuotes() {
  if (!stocks.value.length) return
  const codes = stocks.value.map(s => s.stockCode || s.code).join(',')
  if (!codes) return
  try {
    const res: any = await stockApi.quotes(codes)
    const list = Array.isArray(res) ? res : []
    const map = new Map<string, any>()
    for (const q of list) {
      const qCode = (q.code || q.symbol || '').replace(/^(sh|sz|bj)/, '')
      map.set(qCode, q)
    }
    stocks.value = stocks.value.map(s => {
      const code = (s.stockCode || s.code || '').replace(/^(sh|sz|bj)/, '')
      const q = map.get(code)
      if (!q) return s
      return {
        ...s,
        close: q.close ?? q.price ?? s.close,
        price: q.price ?? q.close ?? s.price,
        open: q.open ?? s.open,
        high: q.high ?? s.high,
        low: q.low ?? s.low,
        changePercent: q.changePercent ?? s.changePercent,
        change: q.change ?? s.change,
        volume: q.volume ?? s.volume,
        turnoverRate: q.turnoverRate ?? s.turnoverRate,
        amount: q.amount ?? s.amount,
      }
    })
  } catch { /* ignore */ }
}

async function handleRefreshQuotes() {
  refreshing.value = true
  try {
    const res: any = await stockApi.watchlist()
    const list = Array.isArray(res) ? res : []
    stocks.value = list
  } catch { /* ignore */ }
  finally {
    refreshing.value = false
  }
}

async function addStock(code: string, name: string) {
  const pureCode = code.replace(/^(sh|sz|bj)/i, '')
  if (stocks.value.some(s => (s.stockCode || s.code) === pureCode)) {
    ElMessage.warning('已在自选股中')
    return
  }
  try {
    await stockApi.watchlistAdd(pureCode, name)
    ElMessage.success('已添加')
    loadList()
  } catch {
    ElMessage.error('添加失败')
  }
}

async function removeStock(row: Record<string, any>) {
  try {
    const id = row.id
    if (id && id > 0) {
      await stockApi.watchlistRemove(id)
    } else {
      await stockApi.watchlistRemove(0, row.stockCode || row.code || '')
    }
    ElMessage.success('已移除')
    loadList()
  } catch {
    ElMessage.error('移除失败')
  }
}

function editStock(row: Record<string, any>) {
  editForm.value = { ...row }
  editVisible.value = true
}

async function saveEdit() {
  if (!editForm.value) return
  saving.value = true
  try {
    await stockApi.watchlistUpdate({
      id: editForm.value.id,
      name: editForm.value.stockName,
      group: editForm.value.groupName,
      sort_order: editForm.value.sortOrder,
      remark: editForm.value.remark,
    })
    ElMessage.success('已更新')
    editVisible.value = false
    loadList()
  } catch {
    ElMessage.error('更新失败')
  } finally {
    saving.value = false
  }
}

function analyzeStock(row: Record<string, any>) {
  const code = row.stockCode || row.code || ''
  const name = row.stockName || row.name || ''
  router.push({ path: '/', query: { code, name } })
}

onMounted(() => {
  loadList()
})

onBeforeUnmount(() => {
  quoteTimer.stop()
})
</script>

<style scoped lang="scss">
.pnl-up { color: #f56c6c; font-weight: 500; }
.pnl-down { color: #67c23a; font-weight: 500; }
</style>
