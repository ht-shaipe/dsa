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
        </el-descriptions>
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
        </div>
      </el-card>

      <el-row :gutter="20" style="margin-bottom: 20px">
        <el-col :span="24">
          <el-card shadow="hover">
            <template #header>市场热点</template>
            <el-row :gutter="16">
              <el-col :xs="24" :sm="12" :md="8" :lg="6" v-for="h in hotspots" :key="h.topic || h.name">
                <el-card shadow="hover" class="hotspot-card" @click="showHotspotDetail(h)">
                  <div class="hotspot-topic">{{ h.topic || h.name }}</div>
                  <div class="hotspot-heat">热度: {{ h.heat || h.score || '-' }}</div>
                  <el-tag v-if="h.category" size="small" style="margin-top:8px">{{ h.category }}</el-tag>
                </el-card>
              </el-col>
            </el-row>
            <el-empty v-if="!hotspots.length" description="暂无热点数据" />
          </el-card>
        </el-col>
      </el-row>

      <el-card shadow="hover">
        <template #header>筛选结果</template>
        <el-table :data="screenResults" stripe style="width:100%" v-loading="screening">
          <el-table-column prop="code" label="代码" width="100" />
          <el-table-column prop="name" label="名称" width="120" />
          <el-table-column prop="strategy" label="策略" width="120" />
          <el-table-column prop="score" label="评分" width="80">
            <template #default="{ row }">
              <span :style="{ color: row.score >= 80 ? '#67c23a' : row.score >= 60 ? '#e6a23c' : '#f56c6c' }">
                {{ row.score }}
              </span>
            </template>
          </el-table-column>
          <el-table-column prop="reason" label="筛选理由" min-width="200" show-overflow-tooltip />
        </el-table>
        <el-empty v-if="!screenResults.length && !screening" description="点击「执行筛选」查看结果" />
      </el-card>
    </template>

    <el-dialog v-model="hotspotDialogVisible" :title="'热点详情 - ' + currentHotspot.topic" width="600px">
      <el-descriptions :column="1" border v-if="hotspotDetail">
        <el-descriptions-item label="主题">{{ hotspotDetail.topic }}</el-descriptions-item>
        <el-descriptions-item label="热度">{{ hotspotDetail.heat || hotspotDetail.score }}</el-descriptions-item>
        <el-descriptions-item label="分类">{{ hotspotDetail.category || '-' }}</el-descriptions-item>
        <el-descriptions-item label="相关股票">
          <el-tag v-for="s in (hotspotDetail.stocks || hotspotDetail.related_stocks || [])" :key="s.code || s" size="small" style="margin:2px">
            {{ s.code || s }} {{ s.name || '' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="描述">{{ hotspotDetail.description || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { screeningApi } from '@/api/screening'

const statusData = ref<Record<string, any>>({})
const statusEnabled = ref(false)
const strategies = ref<any[]>([])
const hotspots = ref<any[]>([])
const screenResults = ref<any[]>([])
const activeStrategy = ref('')
const screening = ref(false)
const hotspotDialogVisible = ref(false)
const currentHotspot = ref<Record<string, any>>({})
const hotspotDetail = ref<Record<string, any> | null>(null)

async function loadStatus() {
  try {
    const res: any = await screeningApi.status()
    statusData.value = res.data || {}
    statusEnabled.value = !!(res.data?.enabled || res.data?.alphaSift?.enabled)
  } catch {
    statusEnabled.value = false
  }
}

async function loadStrategies() {
  try {
    const res: any = await screeningApi.strategies()
    strategies.value = res.data || []
    if (strategies.value.length && !activeStrategy.value) {
      activeStrategy.value = strategies.value[0].id || strategies.value[0].name || ''
    }
  } catch { /* ignore */ }
}

async function loadHotspots() {
  try {
    const res: any = await screeningApi.hotspots()
    hotspots.value = res.data || []
  } catch { /* ignore */ }
}

async function runScreen() {
  screening.value = true
  try {
    const res: any = await screeningApi.screen(activeStrategy.value || undefined)
    screenResults.value = res.data || []
    ElMessage.success(`筛选完成，找到 ${screenResults.value.length} 只股票`)
  } catch {
    ElMessage.error('筛选执行失败')
  } finally {
    screening.value = false
  }
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
    hotspotDetail.value = res.data || h
  } catch {
    hotspotDetail.value = h
  }
}

onMounted(async () => {
  await loadStatus()
  if (statusEnabled.value) {
    loadStrategies()
    loadHotspots()
  }
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
  margin-bottom: 8px;
}
.hotspot-heat {
  font-size: 14px;
  color: var(--el-color-primary);
}
</style>
