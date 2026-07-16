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
            <el-input
              v-model="searchText"
              placeholder="搜索代码/名称"
              clearable
              style="width:180px"
              @keyup.enter="loadList"
              @clear="loadList"
            />
            <el-button :icon="Search" @click="loadList">搜索</el-button>
            <el-button type="primary" @click="showAddDialog = true">添加股票</el-button>
            <el-button type="success" :disabled="taskStore.hasRunningTasks" @click="showInitPoolDialog = true">
              初始化股票池
            </el-button>
            <el-button v-if="selectedIds.length" type="danger" @click="handleBatchRemove">
              批量删除({{ selectedIds.length }})
            </el-button>
          </div>
        </div>
      </template>

      <el-table
        :data="list"
        stripe
        style="width:100%"
        v-loading="loading"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="40" />
        <el-table-column label="代码" width="100">
          <template #default="{ row }">
            <span style="font-weight:500">{{ row.stockCode }}</span>
          </template>
        </el-table-column>
        <el-table-column label="名称" min-width="120">
          <template #default="{ row }">{{ row.stockName || '-' }}</template>
        </el-table-column>
        <el-table-column label="市场" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.marketId === 1 ? 'danger' : 'success'">
              {{ row.marketId === 1 ? '沪' : '深' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="行业" min-width="100">
          <template #default="{ row }">{{ row.industry || '-' }}</template>
        </el-table-column>
        <el-table-column label="状态" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.status === 1 ? 'success' : 'info'">
              {{ row.status === 1 ? '正常' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="添加时间" width="168">
          <template #default="{ row }">{{ formatDateTime(row.createTime) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-popconfirm title="确定删除?" @confirm="handleRemove(row)">
              <template #reference>
                <el-button link type="danger">删除</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>

      <div style="display:flex;justify-content:flex-end;margin-top:16px;gap:8px;align-items:center">
        <el-pagination
          v-model:current-page="page"
          :page-size="pageSize"
          :total="totalCount"
          layout="total, prev, pager, next"
          @current-change="loadList"
        />
      </div>

      <el-empty v-if="!list.length && !loading" description="股票池为空，点击初始化股票池从市场获取" />
    </el-card>

    <!-- 日线数据管理 -->
    <el-card shadow="hover" style="margin-top:16px">
      <template #header>
        <span style="font-size:16px;font-weight:600">日线数据管理</span>
      </template>

      <!-- 操作按钮区 -->
      <div style="display:flex;gap:12px;align-items:center;margin-bottom:20px;flex-wrap:wrap">
        <el-button type="primary" @click="initDailyData" :loading="syncRunning" :disabled="syncRunning">
          {{ syncRunning ? '同步进行中...' : '初始化日线数据' }}
        </el-button>
        <el-button type="danger" @click="cleanDailyData" :loading="cleaning">清理过期数据</el-button>
        <el-button type="success" @click="exportDailyData" :loading="dailyExporting">导出日线数据</el-button>
        <el-button type="warning" @click="triggerImportDailyData" :loading="dailyImporting">导入日线数据</el-button>
        <input ref="importFileInput" type="file" accept=".dsa-daily.json" style="display:none" @change="handleImportFile" />
        <el-button @click="loadSyncConfig">刷新状态</el-button>
      </div>

      <!-- 同步配置区 -->
      <el-form :model="dataSyncForm" label-width="130px" style="max-width:650px">
        <el-divider content-position="left">同步范围</el-divider>
        <el-form-item label="市场板块">
          <el-checkbox-group v-model="dataSyncForm.boards">
            <el-checkbox label="沪市主板" value="sh_main" />
            <el-checkbox label="深市主板" value="sz_main" />
            <el-checkbox label="创业板" value="sz_gem" />
            <el-checkbox label="科创板" value="sh_kj" />
            <el-checkbox label="北交所" value="bj_main" />
          </el-checkbox-group>
        </el-form-item>
        <el-divider content-position="left">风险过滤</el-divider>
        <el-form-item label="排除ST股票">
          <el-switch v-model="dataSyncForm.excludeSt" />
          <span style="margin-left:8px;color:var(--el-text-color-secondary)">过滤名称含ST/*ST的股票</span>
        </el-form-item>
        <el-form-item label="排除退市风险">
          <el-switch v-model="dataSyncForm.excludeDelistingRisk" />
          <span style="margin-left:8px;color:var(--el-text-color-secondary)">过滤名称含退市/退的股票</span>
        </el-form-item>
        <el-form-item label="排除次新股">
          <el-switch v-model="dataSyncForm.excludeNewStock" />
          <span style="margin-left:8px;color:var(--el-text-color-secondary)">上市不足60天的股票波动大、数据少</span>
        </el-form-item>
        <el-divider content-position="left">数据管理</el-divider>
        <el-form-item label="保留天数">
          <el-input-number v-model="dataSyncForm.retentionDays" :min="60" :max="1000" :step="30" style="width:180px" />
          <span style="margin-left:8px;color:var(--el-text-color-secondary)">超过此天数的日线数据将被清理</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveDataSyncConfig" :loading="savingConfig">保存配置</el-button>
        </el-form-item>
      </el-form>

      <!-- 进度卡片 -->
      <el-card v-if="syncStatus.running || syncStatus.total > 0" shadow="never" style="max-width:650px;margin-top:16px">
        <div style="display:flex;align-items:center;gap:12px;margin-bottom:8px">
          <el-tag v-if="syncStatus.paused" type="warning">已暂停</el-tag>
          <el-tag v-else :type="syncStatus.running ? 'warning' : 'success'">
            {{ syncStatus.running ? '同步中' : (syncStatus.phase === 'done' ? '已完成' : '未开始') }}
          </el-tag>
          <span v-if="syncStatus.running && syncStatus.phase" style="color:var(--el-text-color-secondary)">
            {{ syncPhaseLabel }}
          </span>
        </div>
        <el-progress
          v-if="syncStatus.total > 0"
          :percentage="Math.round((syncStatus.done / syncStatus.total) * 100)"
          :status="syncStatus.running ? '' : 'success'"
          :stroke-width="18"
          :format="() => `${syncStatus.done} / ${syncStatus.total}`"
        />
        <div style="margin-top:6px;font-size:12px;color:var(--el-text-color-secondary)">
          <span v-if="syncStatus.running">
            <template v-if="syncStatus.total > 0">已完成 {{ Math.round((syncStatus.done / syncStatus.total) * 100) }}%</template>
            <template v-else>已处理 {{ syncStatus.done }} 条</template>
            <template v-if="estimatedTime">，约需 {{ estimatedTime }}</template>
          </span>
          <span v-else-if="syncStatus.phase === 'done'">全部 {{ syncStatus.total }} 只股票日线数据已同步完成</span>
        </div>
        <div v-if="syncStatus.failed > 0" style="margin-top:4px;color:var(--el-color-danger)">
          失败: {{ syncStatus.failed }}
        </div>
      </el-card>
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
import { Search } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { stockPoolApi } from '@/api/stockPool'
import { systemApi } from '@/api/system'
import { useTaskStore, getPhaseLabel } from '@/stores/task'
import InitStockPoolDialog from '@/components/common/InitStockPoolDialog.vue'

const taskStore = useTaskStore()

// ========== 股票池 ==========
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

function formatDateTime(val: string | null | undefined): string {
  if (!val) return '-'
  try {
    const d = new Date(val)
    if (isNaN(d.getTime())) return val
    return d.toLocaleString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  } catch {
    return val
  }
}

async function loadList() {
  loading.value = true
  try {
    const res = await stockPoolApi.list({
      search: searchText.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    })
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
  try {
    await stockPoolApi.remove({ id: row.id })
    ElMessage.success('已删除')
    loadList()
  } catch (e: any) {
    ElMessage.error(e.message || '删除失败')
  }
}

async function handleBatchRemove() {
  if (!selectedIds.value.length) return
  try {
    const { ElMessageBox } = await import('element-plus')
    await ElMessageBox.confirm(`确定删除选中的 ${selectedIds.value.length} 只股票?`, '批量删除')
    await stockPoolApi.batchRemove(selectedIds.value)
    ElMessage.success('已批量删除')
    selectedIds.value = []
    loadList()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '批量删除失败')
  }
}

async function handleAdd() {
  if (!addForm.value.code.trim()) {
    ElMessage.warning('请输入股票代码')
    return
  }
  addLoading.value = true
  try {
    await stockPoolApi.add({
      code: addForm.value.code.trim(),
      name: addForm.value.name.trim() || undefined,
      industry: addForm.value.industry.trim() || undefined,
    })
    ElMessage.success('添加成功')
    showAddDialog.value = false
    addForm.value = { code: '', name: '', industry: '' }
    loadList()
  } catch (e: any) {
    ElMessage.error(e.message || '添加失败')
  } finally {
    addLoading.value = false
  }
}

function onPoolInitStarted() {
  loadList()
}

// ========== 日线数据管理 ==========
const syncStatus = computed(() => taskStore.tasks['init_daily_data'] || {})
const syncRunning = computed(() => taskStore.hasRunningTasks)
const cleaning = ref(false)
const dailyExporting = ref(false)
const dailyImporting = ref(false)
const savingConfig = ref(false)
const importFileInput = ref<HTMLInputElement | null>(null)

const dataSyncForm = ref({
  boards: ['sh_main', 'sz_main', 'sz_gem'],
  excludeSt: true,
  excludeNewStock: true,
  excludeDelistingRisk: true,
  retentionDays: 120,
})

const syncPhaseLabel = computed(() => getPhaseLabel(syncStatus.value.phase || ''))

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
    if (res?.config) {
      dataSyncForm.value = {
        boards: res.config.boards || ['sh_main', 'sz_main', 'sz_gem'],
        excludeSt: res.config.excludeSt ?? res.config.exclude_st ?? true,
        excludeNewStock: res.config.excludeNewStock ?? res.config.exclude_new_stock ?? true,
        excludeDelistingRisk: res.config.excludeDelistingRisk ?? res.config.exclude_delisting_risk ?? true,
        retentionDays: res.config.retentionDays ?? res.config.retention_days ?? 120,
      }
    }
  } catch { /* ignore */ }
}

async function initDailyData() {
  try {
    const res: any = await systemApi.initDailyData()
    ElMessage.success(res?.message || '同步已启动')
  } catch {
    // error already shown by api interceptor
  }
}

async function cleanDailyData() {
  cleaning.value = true
  try {
    const res: any = await systemApi.cleanDailyData()
    ElMessage.success(`已清理 ${res?.deleted ?? 0} 条过期数据`)
  } catch {
    // error already shown by api interceptor
  } finally {
    cleaning.value = false
  }
}

async function saveDataSyncConfig() {
  savingConfig.value = true
  try {
    await systemApi.save({
      data_sync: {
        boards: dataSyncForm.value.boards,
        exclude_st: dataSyncForm.value.excludeSt,
        exclude_new_stock: dataSyncForm.value.excludeNewStock,
        exclude_delisting_risk: dataSyncForm.value.excludeDelistingRisk,
        retention_days: dataSyncForm.value.retentionDays,
      },
    })
    ElMessage.success('数据同步配置已保存')
  } catch {
    // error already shown by api interceptor
  } finally {
    savingConfig.value = false
  }
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
      const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: 'DSA日线数据', extensions: ['dsa-daily.json'] }],
      })
      if (!filePath) {
        dailyExporting.value = false
        return
      }
      const encoder = new TextEncoder()
      await writeFile(filePath, encoder.encode(jsonStr))
      ElMessage.success(`导出成功: ${res?.stockCount ?? 0} 只股票, ${res?.recordCount ?? 0} 条记录`)
    } else {
      const blob = new Blob([jsonStr], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = defaultName
      a.click()
      URL.revokeObjectURL(url)
      ElMessage.success(`导出成功: ${res?.stockCount ?? 0} 只股票, ${res?.recordCount ?? 0} 条记录`)
    }
  } catch (e: any) {
    if (e?.message !== 'User cancelled') {
      ElMessage.error('导出失败')
    }
  } finally {
    dailyExporting.value = false
  }
}

async function triggerImportDailyData() {
  if (isTauri()) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const { readFile } = await import('@tauri-apps/plugin-fs')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'DSA日线数据', extensions: ['dsa-daily.json'] }],
      })
      if (!selected) return
      const filePath = typeof selected === 'string' ? selected : (selected as any).path
      if (!filePath) return

      dailyImporting.value = true
      const bytes = await readFile(filePath)
      const text = typeof bytes === 'string' ? bytes : new TextDecoder().decode(bytes as Uint8Array)
      await doImportDailyData(text)
    } catch (e: any) {
      if (e?.message !== 'User cancelled') {
        ElMessage.error('导入失败')
      }
    } finally {
      dailyImporting.value = false
    }
  } else {
    importFileInput.value?.click()
  }
}

async function doImportDailyData(text: string) {
  let data: any
  try {
    data = JSON.parse(text)
  } catch {
    ElMessage.error('文件格式错误，请选择有效的 .dsa-daily.json 文件')
    return
  }
  if (!data?.records || !Array.isArray(data.records)) {
    ElMessage.error('文件内容无效，缺少 records 数据')
    return
  }
  const res: any = await systemApi.importDailyData(data)
  ElMessage.success(`导入完成: 成功 ${res?.imported ?? 0} 条, 跳过 ${res?.skipped ?? 0} 条`)
}

async function handleImportFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  input.value = ''

  dailyImporting.value = true
  try {
    const text = await file.text()
    await doImportDailyData(text)
  } catch {
    ElMessage.error('导入失败')
  } finally {
    dailyImporting.value = false
  }
}

// ========== 生命周期 ==========
onMounted(() => {
  loadList()
  stockPoolApi.count().then(res => { totalCount.value = res.total ?? -1 }).catch(() => {})
  loadSyncConfig()
})
</script>

<style scoped lang="scss">
.stock-pool-view {
  padding: 0;
}
</style>
