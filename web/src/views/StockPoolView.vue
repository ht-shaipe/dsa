<template>
  <div class="stock-pool-view">
    <el-card shadow="hover">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px">
          <div style="display:flex;align-items:center;gap:12px">
            <span style="font-size:16px;font-weight:600">股票池</span>
            <el-tag v-if="totalCount >= 0" type="info" size="small">{{ totalCount }} 只</el-tag>
          </div>
          <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">
            <el-input v-model="searchText" placeholder="搜索代码/名称" clearable style="width:180px" @keyup.enter="loadList" @clear="loadList" />
            <el-button :icon="Search" @click="loadList">搜索</el-button>
            <el-button type="primary" @click="showAddDialog = true">添加股票</el-button>
            <el-button type="success" :disabled="taskStore.hasRunningTasks" @click="showInitPoolDialog = true">初始化股票池</el-button>
            <el-dropdown trigger="click" @command="handleDailyCommand">
              <el-button :loading="syncRunning">
                日线数据<el-icon class="el-icon--right"><ArrowDown /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="init" :disabled="syncRunning">
                    <el-icon><Download /></el-icon>
                    {{ syncRunning ? '同步进行中...' : '初始化日线数据' }}
                  </el-dropdown-item>
                  <el-dropdown-item command="clean" divided>
                    <el-icon><Delete /></el-icon>
                    清理过期数据
                  </el-dropdown-item>
                  <el-dropdown-item command="export">
                    <el-icon><Upload /></el-icon>
                    导出日线数据
                  </el-dropdown-item>
                  <el-dropdown-item command="import">
                    <el-icon><Download /></el-icon>
                    导入日线数据
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
            <el-button v-if="selectedIds.length" type="danger" @click="handleBatchRemove">
              批量删除({{ selectedIds.length }})
            </el-button>
          </div>
        </div>
      </template>

      <el-table :data="list" stripe style="width:100%" v-loading="loading" @selection-change="handleSelectionChange" :row-class-name="rowClassName">
        <el-table-column type="selection" width="40" />

        <!-- 代码 / 名称 -->
        <el-table-column label="代码" width="90" fixed="left">
          <template #default="{ row }">
            <span style="font-weight:600;font-family:monospace">{{ row.stockCode }}</span>
          </template>
        </el-table-column>
        <el-table-column label="名称" width="90" fixed="left">
          <template #default="{ row }">
            <span :class="{'st-name': isST(row.stockName)}">{{ row.stockName || '-' }}</span>
          </template>
        </el-table-column>

        <!-- 行情 -->
        <el-table-column label="最新价" width="80" align="right">
          <template #default="{ row }">
            <span v-if="row.latestPrice" style="font-weight:600" :class="priceClass(row.changePercent)">{{ fmt(row.latestPrice) }}</span>
            <span v-else style="color:var(--el-text-color-placeholder)">-</span>
          </template>
        </el-table-column>
        <el-table-column label="涨跌幅" width="96" align="right">
          <template #default="{ row }">
            <span v-if="row.changePercent != null" :class="pctClass(row.changePercent)">
              {{ row.changePercent > 0 ? '+' : '' }}{{ fmt(row.changePercent) }}%
            </span>
            <span v-else style="color:var(--el-text-color-placeholder)">-</span>
          </template>
        </el-table-column>
        <el-table-column label="涨跌额" width="86" align="right">
          <template #default="{ row }">
            <span v-if="row.changePrice != null" :class="priceClass(row.changePercent)">
              {{ row.changePrice > 0 ? '+' : '' }}{{ fmt(row.changePrice) }}
            </span>
            <span v-else style="color:var(--el-text-color-placeholder)">-</span>
          </template>
        </el-table-column>

        <!-- 昨收 / 开盘 / 最高 / 最低 -->
        <el-table-column label="昨收" width="78" align="right">
          <template #default="{ row }">{{ row.previousClose ? fmt(row.previousClose) : '-' }}</template>
        </el-table-column>
        <el-table-column label="开盘" width="78" align="right">
          <template #default="{ row }">{{ row.quoteOpen ? fmt(row.quoteOpen) : '-' }}</template>
        </el-table-column>
        <el-table-column label="最高" width="78" align="right">
          <template #default="{ row }">
            <span v-if="row.quoteHigh" style="color:var(--el-color-danger)">{{ fmt(row.quoteHigh) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="最低" width="78" align="right">
          <template #default="{ row }">
            <span v-if="row.quoteLow" style="color:var(--el-color-success)">{{ fmt(row.quoteLow) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>

        <!-- 成交量 / 成交额 / 换手率 -->
        <el-table-column label="成交量" width="106" align="right">
          <template #default="{ row }">{{ row.volume ? fmtVol(row.volume) : '-' }}</template>
        </el-table-column>
        <el-table-column label="成交额" width="100" align="right">
          <template #default="{ row }">{{ row.amount ? fmtAmount(row.amount) : '-' }}</template>
        </el-table-column>
        <el-table-column label="换手率" width="96" align="right">
          <template #default="{ row }">{{ row.turnoverRatio != null ? fmt(row.turnoverRatio) + '%' : '-' }}</template>
        </el-table-column>

        <!-- 估值 -->
        <el-table-column label="PE" width="78" align="right">
          <template #default="{ row }">
            <span v-if="row.pe != null" :class="peClass(row.pe)">{{ fmt(row.pe) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="PB" width="78" align="right">
          <template #default="{ row }">
            <span v-if="row.pb != null">{{ fmt(row.pb) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="总市值(亿)" width="100" align="right">
          <template #default="{ row }">{{ row.totalMarketCap ? fmt(row.totalMarketCap) : (row.total ? fmt(row.total) : '-') }}</template>
        </el-table-column>
        <el-table-column label="流通市值(亿)" width="106" align="right">
          <template #default="{ row }">{{ row.liquidMarketCap ? fmt(row.liquidMarketCap) : (row.outstanding ? fmt(row.outstanding) : '-') }}</template>
        </el-table-column>

        <!-- 行业 -->
        <el-table-column label="行业" width="90">
          <template #default="{ row }">{{ row.industry || '-' }}</template>
        </el-table-column>

        <!-- 状态 -->
        <el-table-column label="状态" width="68">
          <template #default="{ row }">
            <el-tag size="small" :type="row.status === 1 ? 'success' : 'info'">
              {{ row.status === 1 ? '正常' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-popconfirm title="确定删除?" @confirm="handleRemove(row)">
              <template #reference>
                <el-button link type="danger" size="small">删除</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>

      <div style="display:flex;justify-content:space-between;align-items:center;margin-top:16px;flex-wrap:wrap;gap:8px">
        <div style="display:flex;align-items:center;gap:8px">
          <span style="font-size:13px;color:var(--el-text-color-secondary)">保留天数</span>
          <el-input-number v-model="retentionDays" :min="60" :max="1000" :step="30" size="small" style="width:120px" @change="saveRetentionDays" />
          <el-button v-if="syncStatus.running || syncStatus.total > 0" link type="info" size="small" @click="showSyncProgress = !showSyncProgress">
            {{ syncRunning ? '同步中' : '同步详情' }}
          </el-button>
        </div>
        <el-pagination v-model:current-page="page" :page-size="pageSize" :total="totalCount" layout="total, prev, pager, next" @current-change="loadList" />
      </div>

      <el-collapse-transition>
        <div v-if="showSyncProgress && (syncStatus.running || syncStatus.total > 0)" style="margin-top:12px">
          <el-progress
            v-if="syncStatus.total > 0"
            :percentage="Math.round((syncStatus.done / syncStatus.total) * 100)"
            :status="syncStatus.running ? '' : 'success'"
            :stroke-width="14"
            :format="() => `${syncStatus.done} / ${syncStatus.total}`"
          />
          <div style="margin-top:4px;font-size:12px;color:var(--el-text-color-secondary);display:flex;gap:12px;align-items:center">
            <span v-if="syncStatus.running">
              <template v-if="syncStatus.total > 0">已完成 {{ Math.round((syncStatus.done / syncStatus.total) * 100) }}%</template>
              <template v-else>已处理 {{ syncStatus.done }} 条</template>
              <template v-if="estimatedTime">，约需 {{ estimatedTime }}</template>
            </span>
            <span v-else-if="syncStatus.phase === 'done'">全部 {{ syncStatus.total }} 只股票日线数据已同步</span>
            <span v-if="syncStatus.failed > 0" style="color:var(--el-color-danger)">失败: {{ syncStatus.failed }}</span>
          </div>
        </div>
      </el-collapse-transition>

      <el-empty v-if="!list.length && !loading" description="股票池为空，点击初始化股票池从市场获取" />
    </el-card>

    <el-dialog v-model="showAddDialog" title="添加股票到股票池" width="420px">
      <el-form label-width="80px">
        <el-form-item label="股票代码">
          <el-input v-model="addForm.code" placeholder="如 600519" @keyup.enter="handleAdd" />
        </el-form-item>
        <el-form-item label="股票名称">
          <el-input v-model="addForm.name" placeholder="如 贵州茅台" />
        </el-form-item>
        <el-form-item label="行业">
          <el-input v-model="addForm.industry" placeholder="可选" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showAddDialog = false">取消</el-button>
        <el-button type="primary" :loading="addLoading" @click="handleAdd">添加</el-button>
      </template>
    </el-dialog>

    <InitStockPoolDialog v-model:visible="showInitPoolDialog" @started="onPoolInitStarted" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Search, ArrowDown, Download, Delete, Upload } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { stockPoolApi } from '@/api/stockPool'
import { systemApi } from '@/api/system'
import { useTaskStore } from '@/stores/task'
import InitStockPoolDialog from '@/components/common/InitStockPoolDialog.vue'
import { formatPrice as fmt, formatVolume as fmtVol, priceClass, peClass, isST } from '@/utils/stock'
import { formatMoney as fmtAmount, formatDateTime } from '@/utils/format'

const taskStore = useTaskStore()

// ========== 股票池列表 ==========
const loading = ref(false)
const list = ref<any[]>([])
const totalCount = ref(-1)
const page = ref(1)
const pageSize = ref(50)
const searchText = ref('')
const selectedIds = ref<number[]>([])
const showAddDialog = ref(false)
const addLoading = ref(false)
const addForm = ref({ code: '', name: '', industry: '' })
const showInitPoolDialog = ref(false)

function pctClass(pct: number | null | undefined): string {
  return priceClass(pct).replace('price-', 'pct-')
}
function rowClassName({ row }: { row: any }): string {
  if (!row.changePercent) return ''
  return row.changePercent > 0 ? 'row-up' : row.changePercent < 0 ? 'row-down' : ''
}

async function loadList() {
  loading.value = true
  try {
    const res = await stockPoolApi.list({ search: searchText.value || undefined, page: page.value, page_size: pageSize.value })
    list.value = res.list || []
    totalCount.value = res.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '加载失败')
  } finally {
    loading.value = false
  }
}

function handleSelectionChange(rows: any[]) {
  selectedIds.value = rows.map(r => r.id).filter(Boolean)
}
async function handleRemove(row: any) {
  try { await stockPoolApi.remove({ id: row.id }); ElMessage.success('已删除'); loadList() }
  catch (e: any) { ElMessage.error(e.message || '删除失败') }
}
async function handleBatchRemove() {
  if (!selectedIds.value.length) return
  try {
    const { ElMessageBox } = await import('element-plus')
    await ElMessageBox.confirm(`确定删除选中的 ${selectedIds.value.length} 只股票?`, '批量删除')
    await stockPoolApi.batchRemove(selectedIds.value)
    ElMessage.success('已批量删除'); selectedIds.value = []; loadList()
  } catch (e: any) { if (e !== 'cancel') ElMessage.error(e.message || '批量删除失败') }
}
async function handleAdd() {
  if (!addForm.value.code.trim()) { ElMessage.warning('请输入股票代码'); return }
  addLoading.value = true
  try {
    await stockPoolApi.add({ code: addForm.value.code.trim(), name: addForm.value.name.trim() || undefined, industry: addForm.value.industry.trim() || undefined })
    ElMessage.success('添加成功'); showAddDialog.value = false; addForm.value = { code: '', name: '', industry: '' }; loadList()
  } catch (e: any) { ElMessage.error(e.message || '添加失败') }
  finally { addLoading.value = false }
}
function onPoolInitStarted() { loadList() }

// ========== 日线数据 ==========
const syncStatus = computed(() => taskStore.tasks['init_daily_data'] || {})
const syncRunning = computed(() => taskStore.hasRunningTasks)
const cleaning = ref(false)
const dailyExporting = ref(false)
const dailyImporting = ref(false)
const showSyncProgress = ref(false)
const retentionDays = ref(120)

const estimatedTime = computed(() => {
  const s = syncStatus.value
  if (!s.running || !s.total || s.total < 1 || s.done < 1) return ''
  const remaining = s.total - s.done
  if (remaining <= 0) return ''
  const secsPerItem = s.phase === 'fetching' ? 0.35 : 0.01
  const totalSecs = Math.round(remaining * secsPerItem)
  if (totalSecs < 60) return `${totalSecs} 秒`
  if (totalSecs < 3600) return `${Math.round(totalSecs / 60)} 分钟`
  return `${(totalSecs / 3600).toFixed(1)} 小时`
})

async function loadSyncConfig() {
  try {
    const res: any = await systemApi.syncStatus()
    if (res?.config) retentionDays.value = res.config.retentionDays ?? res.config.retention_days ?? 120
  } catch { /* ignore */ }
}
async function saveRetentionDays() {
  try { await systemApi.save({ data_sync: { retention_days: retentionDays.value } }); ElMessage.success('保留天数已更新') } catch { /* ignore */ }
}
async function handleDailyCommand(cmd: string) {
  switch (cmd) {
    case 'init': return initDailyData()
    case 'clean': return cleanDailyData()
    case 'export': return exportDailyData()
    case 'import': return triggerImportDailyData()
  }
}
async function initDailyData() {
  try { const res: any = await systemApi.initDailyData(); ElMessage.success(res?.message || '同步已启动'); showSyncProgress.value = true }
  catch { /* api interceptor */ }
}
async function cleanDailyData() {
  cleaning.value = true
  try { const res: any = await systemApi.cleanDailyData(); ElMessage.success(`已清理 ${res?.deleted ?? 0} 条过期数据`) }
  catch { /* api interceptor */ } finally { cleaning.value = false }
}
function isTauri(): boolean {
  return typeof window !== 'undefined' && !!((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)
}
async function exportDailyData() {
  dailyExporting.value = true
  try {
    const res: any = await systemApi.exportDailyData()
    const jsonStr = JSON.stringify(res)
    const defaultName = `dsa-daily-${new Date().toISOString().slice(0, 10)}.dsa-daily.json`
    if (isTauri()) {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const { writeFile } = await import('@tauri-apps/plugin-fs')
      const filePath = await save({ defaultPath: defaultName, filters: [{ name: 'DSA日线数据', extensions: ['dsa-daily.json'] }] })
      if (!filePath) { dailyExporting.value = false; return }
      await writeFile(filePath, new TextEncoder().encode(jsonStr))
    } else {
      const blob = new Blob([jsonStr], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a'); a.href = url; a.download = defaultName; a.click(); URL.revokeObjectURL(url)
    }
    ElMessage.success(`导出成功: ${res?.stockCount ?? 0} 只股票, ${res?.recordCount ?? 0} 条记录`)
  } catch (e: any) { if (e?.message !== 'User cancelled') ElMessage.error('导出失败') }
  finally { dailyExporting.value = false }
}
async function triggerImportDailyData() {
  if (isTauri()) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const { readFile } = await import('@tauri-apps/plugin-fs')
      const selected = await open({ multiple: false, filters: [{ name: 'DSA日线数据', extensions: ['dsa-daily.json'] }] })
      if (!selected) return
      const filePath = typeof selected === 'string' ? selected : (selected as any).path
      if (!filePath) return
      dailyImporting.value = true
      const bytes = await readFile(filePath)
      const text = typeof bytes === 'string' ? bytes : new TextDecoder().decode(bytes as Uint8Array)
      await doImportDailyData(text)
    } catch (e: any) { if (e?.message !== 'User cancelled') ElMessage.error('导入失败') }
    finally { dailyImporting.value = false }
  } else { document.getElementById('dsa-daily-import')?.click() }
}
async function doImportDailyData(text: string) {
  let data: any
  try { data = JSON.parse(text) } catch { ElMessage.error('文件格式错误'); return }
  if (!data?.records || !Array.isArray(data.records)) { ElMessage.error('文件内容无效，缺少 records'); return }
  const res: any = await systemApi.importDailyData(data)
  ElMessage.success(`导入完成: 成功 ${res?.imported ?? 0} 条, 跳过 ${res?.skipped ?? 0} 条`)
}

onMounted(() => {
  loadList()
  stockPoolApi.count().then(res => { totalCount.value = res.total ?? -1 }).catch(() => {})
  loadSyncConfig()
})
</script>

<style scoped lang="scss">
.stock-pool-view { padding: 0; }

:deep(.row-up) { background-color: var(--el-color-danger-light-9) !important; }
:deep(.row-down) { background-color: var(--el-color-success-light-9) !important; }

.price-up, .pct-up { color: var(--el-color-danger); font-weight: 600; }
.price-down, .pct-down { color: var(--el-color-success); font-weight: 600; }
.text-warning { color: var(--el-color-warning); }
.st-name { color: var(--el-color-warning); }
</style>
