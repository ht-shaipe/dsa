<template>
  <div class="usage-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>用量概览</span>
          <el-radio-group v-model="period" @change="loadSummary">
            <el-radio-button value="">全部</el-radio-button>
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
            <el-table-column prop="llmProvider" label="供应商" width="120" />
            <el-table-column prop="llmModel" label="模型" width="180" />
            <el-table-column prop="operationType" label="操作类型" width="130" />
            <el-table-column prop="totalPromptTokens" label="输入Token" width="120">
              <template #default="{ row }">{{ Number(row.totalPromptTokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="totalCompletionTokens" label="输出Token" width="120">
              <template #default="{ row }">{{ Number(row.totalCompletionTokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="totalTokens" label="总Token" width="120">
              <template #default="{ row }">{{ Number(row.totalTokens || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="callCount" label="调用次数" width="100">
              <template #default="{ row }">{{ Number(row.callCount || 0).toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="avgLatencyMs" label="平均耗时(ms)" width="120">
              <template #default="{ row }">{{ row.avgLatencyMs != null ? Number(row.avgLatencyMs).toFixed(0) : '-' }}</template>
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
        <el-table-column prop="llmProvider" label="供应商" width="100" />
        <el-table-column prop="llmModel" label="模型" width="150" />
        <el-table-column prop="operationType" label="操作" width="140" />
        <el-table-column prop="stockCode" label="股票" width="90">
          <template #default="{ row }">{{ row.stockCode || '-' }}</template>
        </el-table-column>
        <el-table-column label="输入/输出Token" width="160">
          <template #default="{ row }">
            {{ Number(row.promptTokens || 0).toLocaleString() }} /
            {{ Number(row.completionTokens || 0).toLocaleString() }}
          </template>
        </el-table-column>
        <el-table-column prop="totalTokens" label="总Token" width="100">
          <template #default="{ row }">{{ Number(row.totalTokens || 0).toLocaleString() }}</template>
        </el-table-column>
        <el-table-column prop="cacheHit" label="缓存" width="70">
          <template #default="{ row }">{{ row.cacheHit ? '命中' : '-' }}</template>
        </el-table-column>
        <el-table-column prop="latencyMs" label="耗时(ms)" width="100">
          <template #default="{ row }">{{ row.latencyMs != null ? Number(row.latencyMs).toLocaleString() : '-' }}</template>
        </el-table-column>
        <el-table-column prop="createTime" label="时间" width="170" />
      </el-table>
      <el-empty v-if="!records.length" description="暂无调用记录" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usageApi } from '@/api/usage'

const period = ref('')
const summary = ref<Record<string, any>>({})
const modelBreakdown = ref<any[]>([])
const records = ref<any[]>([])

const summaryCards = computed(() => [
  {
    label: '总Token',
    value: Number(summary.value.totalTokens || 0),
    precision: 0,
  },
  {
    label: '总费用(估算)',
    value: Number(summary.value.totalCostEstimate || 0),
    precision: 4,
    prefix: '¥',
  },
  {
    label: 'API调用次数',
    value: Number(summary.value.totalCalls || 0),
    precision: 0,
    suffix: '次',
  },
])

async function loadSummary() {
  try {
    const res: any = await usageApi.summary(period.value)
    summary.value = res || {}
    modelBreakdown.value = (res || {}).breakdown || []
  } catch { /* ignore */ }
}

async function loadRecords() {
  try {
    const res: any = await usageApi.records({ limit: 50 })
    records.value = Array.isArray(res) ? res : []
  } catch { /* ignore */ }
}

onMounted(() => {
  loadSummary()
  loadRecords()
})
</script>

<style scoped lang="scss">
.stat-card {
  text-align: center;
}
</style>
