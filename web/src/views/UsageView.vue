<template>
  <div class="usage-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>用量概览</span>
          <el-radio-group v-model="period" @change="loadSummary">
            <el-radio-button value="day">日</el-radio-button>
            <el-radio-button value="week">周</el-radio-button>
            <el-radio-button value="month">月</el-radio-button>
          </el-radio-group>
        </div>
      </template>
      <el-row :gutter="20">
        <el-col :span="8" v-for="card in summaryCards" :key="card.label">
          <el-card shadow="hover" class="stat-card">
            <el-statistic :title="card.label" :value="card.value" :precision="card.precision || 0" :prefix="card.prefix || ''" :suffix="card.suffix || ''" />
          </el-card>
        </el-col>
      </el-row>
    </el-card>

    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>模型用量分布</template>
          <el-table :data="modelBreakdown" stripe style="width:100%">
            <el-table-column prop="provider" label="供应商" width="120" />
            <el-table-column prop="model" label="模型" width="180" />
            <el-table-column prop="totalTokens" label="总Token" width="140">
              <template #default="{ row }">{{ Number(row.totalTokens || row.total_tokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="promptTokens" label="输入Token" width="140">
              <template #default="{ row }">{{ Number(row.promptTokens || row.prompt_tokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="completionTokens" label="输出Token" width="140">
              <template #default="{ row }">{{ Number(row.completionTokens || row.completion_tokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="cost" label="费用" width="120">
              <template #default="{ row }">¥{{ Number(row.cost || 0).toFixed(4) }}</template>
            </el-table-column>
            <el-table-column prop="apiCalls" label="API调用" width="100">
              <template #default="{ row }">{{ Number(row.apiCalls || row.api_calls || 0).toLocaleString() }}</template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!modelBreakdown.length" description="暂无模型用量数据" />
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover">
      <template #header>最近调用记录</template>
      <el-table :data="records" stripe style="width:100%">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="provider" label="供应商" width="120" />
        <el-table-column prop="model" label="模型" width="180" />
        <el-table-column prop="method" label="方法" width="140">
          <template #default="{ row }">{{ row.method || row.module + '.' + row.action || '-' }}</template>
        </el-table-column>
        <el-table-column label="Token用量" width="140">
          <template #default="{ row }">
            {{ Number(row.promptTokens || row.prompt_tokens || 0).toLocaleString() }} /
            {{ Number(row.completionTokens || row.completion_tokens || 0).toLocaleString() }}
          </template>
        </el-table-column>
        <el-table-column prop="cost" label="费用" width="120">
          <template #default="{ row }">¥{{ Number(row.cost || 0).toFixed(4) }}</template>
        </el-table-column>
        <el-table-column prop="duration" label="耗时(ms)" width="100">
          <template #default="{ row }">{{ row.duration || '-' }}</template>
        </el-table-column>
        <el-table-column prop="createdAt" label="时间" width="180">
          <template #default="{ row }">{{ row.createdAt || row.created_at || '-' }}</template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!records.length" description="暂无调用记录" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usageApi } from '@/api/usage'

const period = ref('day')
const summary = ref<Record<string, any>>({})
const modelBreakdown = ref<any[]>([])
const records = ref<any[]>([])

const summaryCards = computed(() => [
  {
    label: '总Token',
    value: Number(summary.value.totalTokens || summary.value.total_tokens || 0),
    precision: 0,
  },
  {
    label: '总费用',
    value: Number(summary.value.totalCost || summary.value.total_cost || 0),
    precision: 4,
    prefix: '¥',
  },
  {
    label: 'API调用次数',
    value: Number(summary.value.apiCalls || summary.value.api_calls || 0),
    precision: 0,
    suffix: '次',
  },
])

async function loadSummary() {
  try {
    const res: any = await usageApi.summary(period.value)
    summary.value = res.data || {}
  } catch { /* ignore */ }
}

async function loadDashboard() {
  try {
    const res: any = await usageApi.dashboard()
    const d = res.data || {}
    modelBreakdown.value = d.models || d.modelBreakdown || d.model_breakdown || []
  } catch { /* ignore */ }
}

async function loadRecords() {
  try {
    const res: any = await usageApi.records({ limit: 50 })
    records.value = res.data || []
  } catch { /* ignore */ }
}

onMounted(() => {
  loadSummary()
  loadDashboard()
  loadRecords()
})
</script>

<style scoped lang="scss">
.stat-card {
  text-align: center;
}
</style>
