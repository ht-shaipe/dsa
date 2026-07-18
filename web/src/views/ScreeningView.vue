<template>
  <div class="screening-view">
    

    <template v-if="statusEnabled">
      <el-row :gutter="20" style="margin-bottom: 20px">
        <el-col :span="24">
          <el-card shadow="hover">
            <template #header>市场热点</template>
            <template v-if="hotspotsLoading">
              <div class="hotspot-grid">
                <el-skeleton v-for="i in 12" :key="i" :rows="1" animated style="width: 120px" />
              </div>
            </template>
            <template v-else>
              <div class="hotspot-grid">
                <div
                  v-for="h in hotspots"
                  :key="h.code || h.name"
                  class="hotspot-item"
                  :class="Number(h.changePercent || 0) >= 0 ? 'pnl-up' : 'pnl-down'"
                  @click="showHotspotDetail(h)"
                >
                  <span class="hotspot-item-name">{{ h.name }}</span>
                  <span class="hotspot-item-pct">
                    {{ Number(h.changePercent || 0) >= 0 ? '+' : '' }}{{ Number(h.changePercent || 0).toFixed(2) }}%
                  </span>
                </div>
              </div>
              <el-empty v-if="!hotspots.length" description="暂无热点数据" />
            </template>
          </el-card>
        </el-col>
      </el-row>

      <el-card shadow="hover">
        <template #header>
          <div style="display:flex;justify-content:space-between;align-items:center">
            <el-tabs v-model="activeStrategy" @tab-change="onStrategyChange" style="margin-bottom:-10px">
              <el-tab-pane v-for="s in strategies" :key="s.id || s.name" :label="s.name || s.label" :name="s.id || s.name" />
            </el-tabs>
            <div style="display:flex;gap:8px;align-items:center">
              <el-button v-if="activeStrategy === 'macd_golden_cross'" text type="info" @click="showHistoryDialog = true">
                <el-icon style="margin-right:4px"><Clock /></el-icon>历史
              </el-button>
              <el-button type="primary" :loading="screening" @click="runScreen">
                <el-icon style="margin-right:4px"><CaretRight /></el-icon>执行筛选
              </el-button>
            </div>
          </div>
        </template>

        <el-alert
          v-if="activeStrategy === 'macd_golden_cross' && !dailyDataReady"
          type="warning"
          :closable="false"
          style="margin-bottom:12px"
        >
          MACD策略需要历史日线数据，请先在上方同步日线数据
        </el-alert>

        <div v-if="activeStrategy === 'macd_golden_cross' && currentStrategyParams" style="margin-bottom:16px">
          <el-form :inline="true" size="small" @submit.prevent>
            <el-form-item v-for="(pMeta, pKey) in currentStrategyParams" :key="pKey" :label="pMeta.label || pKey">
              <el-select v-if="pMeta.type === 'select'" v-model="macdParams[pKey]" style="width:120px">
                <el-option v-for="opt in pMeta.options" :key="opt" :label="opt" :value="opt" />
              </el-select>
              <el-input-number v-else-if="pMeta.type === 'integer'" v-model="macdParams[pKey]" :min="pMeta.min" :max="pMeta.max" :step="1" style="width:130px" />
              <el-input-number v-else-if="pMeta.type === 'number'" v-model="macdParams[pKey]" :min="pMeta.min" :max="pMeta.max" :step="pMeta.step || 0.01" :precision="2" style="width:130px" />
            </el-form-item>
            <el-form-item>
              <el-button size="small" @click="resetMacdParams">重置参数</el-button>
            </el-form-item>
          </el-form>
        </div>

        <el-table :data="screenResults" stripe style="width:100%" v-loading="screening" @row-click="onRowClick" class="clickable-table">
          <el-table-column :prop="colKey" :label="colLabel" v-for="{key: colKey, label: colLabel} in resultColumns" :key="colKey" :width="colKey === '代码' || colKey === 'code' ? 100 : colKey === '名称' || colKey === 'name' ? 120 : undefined">
            <template #default="{ row }" v-if="colKey === '涨跌幅' || colKey === 'change_pct' || colKey === 'pct_chg'">
              <span :style="{ color: (row[colKey] || 0) >= 0 ? '#f56c6c' : '#67c23a' }">
                {{ typeof row[colKey] === 'number' ? row[colKey].toFixed(2) : row[colKey] }}{{ typeof row[colKey] === 'number' ? '%' : '' }}
              </span>
            </template>
            <template #default="{ row }" v-else-if="['ma60','dif','dea','macd_hist','close','above_ma60_pct'].includes(colKey)">
              {{ typeof row[colKey] === 'number' ? row[colKey].toFixed(3) : row[colKey] }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80" v-if="screenResults.length">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click.stop="goKline(row)">K线</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-if="!screenResults.length && !screening" description="选择策略并点击「执行筛选」查看结果" />
      </el-card>
    </template>

    <el-card shadow="hover" style="margin-top: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>筛选引擎状态</span>
          <el-tag :type="statusEnabled ? 'success' : 'info'">
            {{ statusEnabled ? '已启用' : '未启用' }}
          </el-tag>
        </div>
      </template>
      <div v-if="statusEnabled">
        <el-descriptions :column="3" border>
          <el-descriptions-item label="状态">{{ statusData.status || '正常' }}</el-descriptions-item>
          <el-descriptions-item label="策略数">{{ strategies.length }}</el-descriptions-item>
          <el-descriptions-item label="热点数">{{ hotspots.length }}</el-descriptions-item>
          <el-descriptions-item label="日线数据">
            <template v-if="dailyDataReady">
              <el-tag type="success" size="small">已就绪</el-tag>
            </template>
            <template v-else>
              <el-tag type="warning" size="small">未同步</el-tag>
              <el-button type="primary" size="small" :loading="syncing" @click="startSync" style="margin-left:8px">
                {{ syncing ? '同步中...' : '同步日线数据' }}
              </el-button>
            </template>
          </el-descriptions-item>
        </el-descriptions>
        <div v-if="syncProgress.running" style="margin-top:12px">
          <el-progress
            :percentage="syncProgress.total > 0 ? Math.round(syncProgress.done / syncProgress.total * 100) : 0"
            :status="syncProgress.phase === 'done' ? 'success' : syncProgress.paused ? 'warning' : undefined"
            :format="() => `${syncProgress.done} / ${syncProgress.total} (失败: ${syncProgress.failed})`"
          />
          <div style="font-size:12px;color:var(--el-text-color-secondary);margin-top:4px">
            {{ getPhaseLabel(syncProgress.phase) }}
            <template v-if="syncProgress.paused">（已暂停）</template>
          </div>
        </div>
      </div>
      <el-empty v-else description="AlphaSift 筛选引擎未启用，请在设置中配置" />
    </el-card>

    <el-dialog v-model="hotspotDialogVisible" :title="'热点详情 - ' + (currentHotspot.name || '')" width="600px">
      <template v-if="hotspotDetailLoading">
        <el-skeleton :rows="5" animated />
        <div style="margin-top:12px">
          <el-skeleton :rows="3" animated />
        </div>
      </template>
      <template v-else-if="hotspotDetail">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="板块名称">{{ hotspotDetail.name || '-' }}</el-descriptions-item>
          <el-descriptions-item v-if="hotspotDetail.code" label="板块代码">{{ hotspotDetail.code }}</el-descriptions-item>
          <el-descriptions-item label="涨跌幅">
            <span :class="Number(hotspotDetail.changePercent || 0) >= 0 ? 'pnl-up' : 'pnl-down'">
              {{ Number(hotspotDetail.changePercent || 0) >= 0 ? '+' : '' }}{{ Number(hotspotDetail.changePercent || 0).toFixed(2) }}%
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="换手率">{{ hotspotDetail.turnoverRate != null ? Number(hotspotDetail.turnoverRate).toFixed(2) + '%' : '-' }}</el-descriptions-item>
          <el-descriptions-item label="上涨家数">{{ hotspotDetail.upCount ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="下跌家数">{{ hotspotDetail.downCount ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="板块类型">
            <el-tag :type="hotspotDetail.sectorType === 'concept' ? 'warning' : 'primary'" size="small">
              {{ hotspotDetail.sectorType === '概念板块' ? '概念板块' : '行业板块' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="描述">{{ hotspotDetail.description || '-' }}</el-descriptions-item>
        </el-descriptions>
        <div v-if="hotspotDetail?.sectors?.length" style="margin-top:12px">
          <el-divider content-position="left">相关板块</el-divider>
          <el-table :data="hotspotDetail.sectors" size="small" stripe>
          <el-table-column prop="name" label="板块名称" min-width="120" />
          <el-table-column label="涨跌幅" width="100">
            <template #default="{ row }">
              <span :class="Number(row.changePercent || 0) >= 0 ? 'pnl-up' : 'pnl-down'">
                {{ Number(row.changePercent || 0) >= 0 ? '+' : '' }}{{ Number(row.changePercent || 0).toFixed(2) }}%
              </span>
            </template>
          </el-table-column>
          <el-table-column label="涨跌家数" width="120">
            <template #default="{ row }">{{ row.upCount ?? '-' }} / {{ row.downCount ?? '-' }}</template>
          </el-table-column>
        </el-table>
        </div>
      </template>
    </el-dialog>

    <el-dialog v-model="showHistoryDialog" title="筛选历史" width="700px">
      <div v-loading="historyLoading">
        <el-table :data="historyList" stripe size="small" @row-click="onHistoryRowClick" style="cursor:pointer">
          <el-table-column prop="strategy" label="策略" width="140" />
          <el-table-column prop="count" label="结果数" width="80" />
          <el-table-column prop="run_time" label="执行时间" min-width="160" />
          <el-table-column label="参数" min-width="120">
            <template #default="{ row }">
              <span style="font-size:12px;color:var(--el-text-color-secondary)">{{ formatParams(row.params_json) }}</span>
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-if="!historyList.length && !historyLoading" description="暂无筛选历史" />
      </div>
      <template #footer>
        <el-button @click="showHistoryDialog = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showHistoryDetailDialog" :title="'筛选详情 - ' + historyDetailBatchId" width="800px">
      <div v-loading="historyDetailLoading">
        <el-table :data="historyDetailResults" stripe size="small">
          <el-table-column prop="code" label="代码" width="100" />
          <el-table-column prop="name" label="名称" width="120" />
          <el-table-column prop="close" label="收盘价" width="80">
            <template #default="{ row }">{{ typeof row.close === 'number' ? row.close.toFixed(2) : row.close }}</template>
          </el-table-column>
          <el-table-column prop="dif" label="DIF" width="80">
            <template #default="{ row }">{{ typeof row.dif === 'number' ? row.dif.toFixed(3) : row.dif }}</template>
          </el-table-column>
          <el-table-column prop="dea" label="DEA" width="80">
            <template #default="{ row }">{{ typeof row.dea === 'number' ? row.dea.toFixed(3) : row.dea }}</template>
          </el-table-column>
          <el-table-column prop="macd_hist" label="MACD柱" width="80">
            <template #default="{ row }">{{ typeof row.macd_hist === 'number' ? row.macd_hist.toFixed(3) : row.macd_hist }}</template>
          </el-table-column>
          <el-table-column label="操作" width="60">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click.stop="goKline(row)">K线</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
      <template #footer>
        <el-button @click="showHistoryDetailDialog = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, reactive } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

const COLUMN_MAP: Record<string, string> = {
  '代码': '代码', 'code': '代码',
  '名称': '名称', 'name': '名称',
  '最新价': '最新价', 'price': '最新价', 'close': '收盘价',
  '涨跌幅': '涨跌幅', 'change_pct': '涨跌幅', 'pct_chg': '涨跌幅',
  '换手率': '换手率', 'turnover_rate': '换手率',
  '市盈率-动态': '市盈率', 'pe': '市盈率',
  '市净率': '市净率', 'pb': '市净率',
  '量比': '量比', 'volume_ratio': '量比',
  '成交额': '成交额', 'amount': '成交额',
  'ma60': 'MA60',
  'dif': 'DIF',
  'dea': 'DEA',
  'macd_hist': 'MACD柱',
  'above_ma60_pct': '高于MA60%',
  'strategy': '',
  'batch_id': '',
}

const resultColumns = computed(() => {
  if (!screenResults.value.length) return []
  const first = screenResults.value[0]
  const seen = new Set<string>()
  return Object.keys(first).map(k => {
    const label = COLUMN_MAP[k] || k
    if (seen.has(label) || label === '') return null
    seen.add(label)
    return { key: k, label }
  }).filter(Boolean) as { key: string; label: string }[]
})

const activeStrategyLabel = computed(() => {
  const s = strategies.value.find(s => (s.id || s.name) === activeStrategy.value)
  return s?.name || activeStrategy.value
})

const currentStrategyParams = computed(() => {
  const s = strategies.value.find(s => (s.id || s.name) === activeStrategy.value)
  return s?.parameters || null
})

import { ElMessage } from 'element-plus'
import { CaretRight, Clock } from '@element-plus/icons-vue'
import { screeningApi } from '@/api/screening'
import { useTaskStore, getPhaseLabel } from '@/stores/task'

const taskStore = useTaskStore()
const statusData = ref<Record<string, any>>({})
const statusEnabled = ref(false)
const dailyDataReady = ref(false)
const strategies = ref<any[]>([])
const hotspots = ref<any[]>([])
const hotspotsLoading = ref(false)
const screenResults = ref<any[]>([])
const activeStrategy = ref('')
const screening = ref(false)
const syncing = computed(() => taskStore.tasks['sync_daily']?.running || false)
const syncProgress = computed(() => taskStore.tasks['sync_daily'] || { running: false, paused: false, total: 0, done: 0, failed: 0, phase: '' })
const hotspotDialogVisible = ref(false)
const currentHotspot = ref<Record<string, any>>({})
const hotspotDetail = ref<Record<string, any> | null>(null)
const hotspotDetailLoading = ref(false)

const macdParams = reactive<Record<string, any>>({
  lookback: 5,
  hist_lookback: 10,
  dif_threshold: 0,
  dea_threshold: 0,
  ma_period: 'ma60',
})

const showHistoryDialog = ref(false)
const historyLoading = ref(false)
const historyList = ref<any[]>([])
const showHistoryDetailDialog = ref(false)
const historyDetailLoading = ref(false)
const historyDetailResults = ref<any[]>([])
const historyDetailBatchId = ref('')

function resetMacdParams() {
  if (!currentStrategyParams.value) return
  for (const [k, v] of Object.entries(currentStrategyParams.value as Record<string, any>)) {
    if (v.default !== undefined) macdParams[k] = v.default
  }
}

function formatParams(paramsJson: string): string {
  if (!paramsJson || paramsJson === '{}') return '默认'
  try {
    const obj = JSON.parse(paramsJson)
    const parts: string[] = []
    if (obj.lookback !== undefined && obj.lookback !== 5) parts.push(`回看${obj.lookback}天`)
    if (obj.hist_lookback !== undefined && obj.hist_lookback !== 10) parts.push(`柱${obj.hist_lookback}条`)
    if (obj.dif_threshold !== undefined && obj.dif_threshold !== 0) parts.push(`DIF>${obj.dif_threshold}`)
    if (obj.dea_threshold !== undefined && obj.dea_threshold !== 0) parts.push(`DEA>${obj.dea_threshold}`)
    if (obj.ma_period && obj.ma_period !== 'ma60') parts.push(obj.ma_period.toUpperCase())
    return parts.length ? parts.join(', ') : '默认'
  } catch {
    return paramsJson
  }
}

let _cache: {
  status?: { data: Record<string, any>; enabled: boolean; dailyReady: boolean; ts: number }
  strategies?: { data: any[]; ts: number }
  hotspots?: { data: any[]; ts: number }
} = {}

const CACHE_TTL = 5 * 60 * 1000

function isCacheValid(entry?: { ts: number }): boolean {
  return !!entry && Date.now() - entry.ts < CACHE_TTL
}

async function loadStatus() {
  if (isCacheValid(_cache.status)) {
    const c = _cache.status!
    statusData.value = c.data
    statusEnabled.value = c.enabled
    dailyDataReady.value = c.dailyReady
    return
  }
  try {
    const res: any = await screeningApi.status()
    statusData.value = res || {}
    statusEnabled.value = !!(res?.enabled || res?.alphaSift?.enabled)
    dailyDataReady.value = !!(res?.dailyDataReady)
    _cache.status = { data: statusData.value, enabled: statusEnabled.value, dailyReady: dailyDataReady.value, ts: Date.now() }
  } catch {
    statusEnabled.value = false
  }
}

async function loadStrategies() {
  if (isCacheValid(_cache.strategies)) {
    strategies.value = _cache.strategies!.data
    if (strategies.value.length && !activeStrategy.value) {
      activeStrategy.value = strategies.value[0].id || strategies.value[0].name || ''
    }
    return
  }
  try {
    const res: any = await screeningApi.strategies()
    strategies.value = Array.isArray(res) ? res : []
    if (strategies.value.length && !activeStrategy.value) {
      activeStrategy.value = strategies.value[0].id || strategies.value[0].name || ''
    }
    if (currentStrategyParams.value) {
      resetMacdParams()
    }
    _cache.strategies = { data: strategies.value, ts: Date.now() }
  } catch { /* ignore */ }
}

async function loadHotspots() {
  if (isCacheValid(_cache.hotspots)) {
    hotspots.value = _cache.hotspots!.data
    return
  }
  hotspotsLoading.value = true
  try {
    const res: any = await screeningApi.hotspots()
    hotspots.value = Array.isArray(res) ? res : []
    _cache.hotspots = { data: hotspots.value, ts: Date.now() }
  } catch { /* ignore */ }
  finally { hotspotsLoading.value = false }
}

async function runScreen() {
  screening.value = true
  try {
    const params: Record<string, any> = {}
    if (activeStrategy.value) params.strategy = activeStrategy.value
    if (activeStrategy.value === 'macd_golden_cross') {
      params.macd_params = { ...macdParams }
    }
    const res: any = await screeningApi.screen(params.strategy, params.macd_params)
    screenResults.value = res?.results || (Array.isArray(res) ? res : [])
    const count = res?.count ?? screenResults.value.length
    ElMessage.success(`筛选完成，找到 ${count} 只股票`)
  } catch(e: any) {
    ElMessage.error(e?.message || '筛选执行失败')
  } finally {
    screening.value = false
  }
}

async function startSync() {
  try {
    await screeningApi.syncDaily()
    ElMessage.success('日线数据同步已启动')
  } catch(e: any) {
    ElMessage.error(e?.message || '同步启动失败')
  }
}

function onStrategyChange() {
  screenResults.value = []
  if (currentStrategyParams.value) {
    resetMacdParams()
  }
}

function onRowClick(row: any) {
  // just highlight, action via button
}

function goKline(row: any) {
  const code = row.code || row.代码 || ''
  const name = row.name || row.名称 || ''
  if (!code) return
  router.push({ path: '/kline', query: { code, name, showMACD: 'true' } })
}

async function loadHistory() {
  historyLoading.value = true
  try {
    const res: any = await screeningApi.history('macd_golden_cross', 20)
    historyList.value = Array.isArray(res) ? res : []
  } catch { historyList.value = [] }
  finally { historyLoading.value = false }
}

async function onHistoryRowClick(row: any) {
  if (!row.batch_id) return
  historyDetailBatchId.value = row.batch_id
  showHistoryDetailDialog.value = true
  historyDetailLoading.value = true
  historyDetailResults.value = []
  try {
    const res: any = await screeningApi.historyDetail(row.batch_id)
    historyDetailResults.value = res?.results || []
  } catch { historyDetailResults.value = [] }
  finally { historyDetailLoading.value = false }
}

async function showHotspotDetail(h: Record<string, any>) {
  currentHotspot.value = h
  hotspotDialogVisible.value = true
  hotspotDetailLoading.value = true
  hotspotDetail.value = null
  try {
    const topic = h.topic || h.name || ''
    const res: any = await screeningApi.hotspotDetail(topic)
    hotspotDetail.value = res || h
  } catch {
    hotspotDetail.value = h
  } finally {
    hotspotDetailLoading.value = false
  }
}

onMounted(async () => {
  await loadStatus()
  if (statusEnabled.value) {
    loadStrategies()
    loadHotspots()
  }
})

import { watch } from 'vue'
watch(showHistoryDialog, (v) => {
  if (v) loadHistory()
})
</script>

<style scoped lang="scss">
.hotspot-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.hotspot-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 14px;
  background: var(--el-fill-color-light);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;

  &:hover {
    background: var(--el-fill-color);
    transform: translateY(-1px);
  }
}
.hotspot-item-name {
  font-size: 13px;
  font-weight: 500;
}
.hotspot-item-pct {
  font-size: 12px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.pnl-up { color: #f56c6c; font-weight: 500; }
.pnl-down { color: #67c23a; font-weight: 500; }
.clickable-table {
  :deep(tbody tr) { cursor: pointer; }
}
</style>
