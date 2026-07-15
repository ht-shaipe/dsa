<template>
  <div class="screening-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
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
                同步日线数据
              </el-button>
            </template>
          </el-descriptions-item>
        </el-descriptions>
        <div v-if="syncProgress.running" style="margin-top:12px">
          <el-progress
            :percentage="syncProgress.total > 0 ? Math.round(syncProgress.done / syncProgress.total * 100) : 0"
            :status="syncProgress.phase === 'done' ? 'success' : undefined"
            :format="() => `${syncProgress.done} / ${syncProgress.total} (失败: ${syncProgress.failed})`"
          />
          <div style="font-size:12px;color:var(--el-text-color-secondary);margin-top:4px">
            {{ syncProgress.phase === 'fetching' ? '正在拉取日线数据...' : syncProgress.phase === 'calculating_indicators' ? '正在计算技术指标...' : syncProgress.phase === 'done' ? '同步完成' : syncProgress.phase }}
          </div>
        </div>
      </div>
      <el-empty v-else description="AlphaSift 筛选引擎未启用，请在设置中配置" />
    </el-card>

    <template v-if="statusEnabled">
      <el-card shadow="hover" style="margin-bottom: 20px">
        <template #header>筛选策略</template>
        <el-tabs v-model="activeStrategy" @tab-change="onStrategyChange">
          <el-tab-pane v-for="s in strategies" :key="s.id || s.name" :label="s.name || s.label" :name="s.id || s.name" />
        </el-tabs>
        <div style="margin-top: 16px">
          <el-button type="primary" :loading="screening" @click="runScreen">
            执行筛选
          </el-button>
          <el-alert
            v-if="activeStrategy === 'macd_golden_cross' && !dailyDataReady"
            type="warning"
            :closable="false"
            style="margin-top:8px"
          >
            MACD策略需要历史日线数据，请先点击上方「同步日线数据」按钮
          </el-alert>
        </div>
      </el-card>

      <el-row :gutter="20" style="margin-bottom: 20px">
        <el-col :span="24">
          <el-card shadow="hover">
            <template #header>市场热点</template>
            <template v-if="hotspotsLoading">
              <el-row :gutter="16">
                <el-col :xs="24" :sm="12" :md="8" :lg="6" v-for="i in 8" :key="i">
                  <el-card shadow="hover" class="hotspot-card">
                    <el-skeleton :rows="3" animated />
                  </el-card>
                </el-col>
              </el-row>
            </template>
            <template v-else>
              <el-row :gutter="16">
                <el-col :xs="24" :sm="12" :md="8" :lg="6" v-for="h in hotspots" :key="h.code || h.name">
                  <el-card shadow="hover" class="hotspot-card" @click="showHotspotDetail(h)">
                    <div class="hotspot-topic">{{ h.name }}</div>
                    <div class="hotspot-code" v-if="h.code">{{ h.code }}</div>
                    <div class="hotspot-stats">
                      <span :class="Number(h.changePercent || 0) >= 0 ? 'pnl-up' : 'pnl-down'">
                        {{ Number(h.changePercent || 0) >= 0 ? '+' : '' }}{{ Number(h.changePercent || 0).toFixed(2) }}%
                      </span>
                      <span v-if="h.upCount != null && h.downCount != null" class="hotspot-counts">
                        {{ h.upCount }}涨 / {{ h.downCount }}跌
                      </span>
                    </div>
                    <el-tag v-if="h.sectorType" :type="h.sectorType === 'concept' ? 'warning' : 'primary'" size="small" style="margin-top:6px">
                      {{ h.sectorType === 'concept' ? '概念' : '行业' }}
                    </el-tag>
                  </el-card>
                </el-col>
              </el-row>
              <el-empty v-if="!hotspots.length" description="暂无热点数据" />
            </template>
          </el-card>
        </el-col>
      </el-row>

      <el-card shadow="hover">
        <template #header>筛选结果</template>
        <el-table :data="screenResults" stripe style="width:100%" v-loading="screening">
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
          <el-table-column label="策略" width="120" v-if="screenResults.length">
            <template #default>{{ activeStrategyLabel }}</template>
          </el-table-column>
        </el-table>
        <el-empty v-if="!screenResults.length && !screening" description="点击「执行筛选」查看结果" />
      </el-card>
    </template>

    <el-dialog v-model="hotspotDialogVisible" :title="'热点详情 - ' + (currentHotspot.name || '')" width="600px">
      <el-descriptions :column="2" border v-if="hotspotDetail">
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
            {{ hotspotDetail.sectorType === 'concept' ? '概念板块' : '行业板块' }}
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
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'

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
import { ElMessage } from 'element-plus'
import { screeningApi } from '@/api/screening'

const statusData = ref<Record<string, any>>({})
const statusEnabled = ref(false)
const dailyDataReady = ref(false)
const strategies = ref<any[]>([])
const hotspots = ref<any[]>([])
const hotspotsLoading = ref(false)
const screenResults = ref<any[]>([])
const activeStrategy = ref('')
const screening = ref(false)
const syncing = ref(false)
const syncProgress = ref<Record<string, any>>({ running: false, total: 0, done: 0, failed: 0, phase: '' })
const hotspotDialogVisible = ref(false)
const currentHotspot = ref<Record<string, any>>({})
const hotspotDetail = ref<Record<string, any> | null>(null)
let progressTimer: ReturnType<typeof setInterval> | null = null

async function loadStatus() {
  try {
    const res: any = await screeningApi.status()
    statusData.value = res || {}
    statusEnabled.value = !!(res?.enabled || res?.alphaSift?.enabled)
    dailyDataReady.value = !!(res?.dailyDataReady)
  } catch {
    statusEnabled.value = false
  }
}

async function loadStrategies() {
  try {
    const res: any = await screeningApi.strategies()
    strategies.value = Array.isArray(res) ? res : []
    if (strategies.value.length && !activeStrategy.value) {
      activeStrategy.value = strategies.value[0].id || strategies.value[0].name || ''
    }
  } catch { /* ignore */ }
}

async function loadHotspots() {
  hotspotsLoading.value = true
  try {
    const res: any = await screeningApi.hotspots()
    hotspots.value = Array.isArray(res) ? res : []
  } catch { /* ignore */ }
  finally { hotspotsLoading.value = false }
}

async function runScreen() {
  screening.value = true
  try {
    const res: any = await screeningApi.screen(activeStrategy.value || undefined)
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
  syncing.value = true
  try {
    await screeningApi.syncDaily()
    ElMessage.success('日线数据同步已启动')
    startProgressPolling()
  } catch(e: any) {
    ElMessage.error(e?.message || '同步启动失败')
    syncing.value = false
  }
}

function startProgressPolling() {
  if (progressTimer) clearInterval(progressTimer)
  progressTimer = setInterval(async () => {
    try {
      const res: any = await screeningApi.syncProgress()
      syncProgress.value = res || {}
      if (!res?.running) {
        if (progressTimer) { clearInterval(progressTimer); progressTimer = null }
        syncing.value = false
        dailyDataReady.value = true
        if (res?.phase === 'done') {
          ElMessage.success('日线数据同步完成')
        }
      }
    } catch { /* ignore */ }
  }, 3000)
}

function onStrategyChange() {
  screenResults.value = []
}

async function showHotspotDetail(h: Record<string, any>) {
  currentHotspot.value = h
  hotspotDialogVisible.value = true
  try {
    const topic = h.topic || h.name || ''
    const res: any = await screeningApi.hotspotDetail(topic)
    hotspotDetail.value = res || h
  } catch {
    hotspotDetail.value = h
  }
}

onMounted(async () => {
  await loadStatus()
  if (statusEnabled.value) {
    loadStrategies()
    loadHotspots()
    const res: any = await screeningApi.syncProgress().catch(() => ({} as any))
    if (res?.running) {
      syncing.value = true
      syncProgress.value = res
      startProgressPolling()
    }
  }
})

onBeforeUnmount(() => {
  if (progressTimer) { clearInterval(progressTimer); progressTimer = null }
})
</script>

<style scoped lang="scss">
.hotspot-card {
  margin-bottom: 16px;
  cursor: pointer;
  transition: transform 0.2s;
  &:hover {
    transform: translateY(-2px);
  }
}
.hotspot-topic {
  font-size: 16px;
  font-weight: 500;
  margin-bottom: 4px;
}
.hotspot-code {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 8px;
}
.hotspot-heat {
  font-size: 14px;
  color: var(--el-color-primary);
}
.hotspot-stats {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
}
.hotspot-counts {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.pnl-up { color: #f56c6c; font-weight: 500; }
.pnl-down { color: #67c23a; font-weight: 500; }
</style>
