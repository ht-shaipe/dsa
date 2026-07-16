<template>
  <div class="analysis-history-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:12px">
          <span>分析历史</span>
          <div style="display:flex;gap:12px;align-items:center;flex-wrap:wrap">
            <StockAutocomplete @select="onFilterStock" style="width:200px" />
            <el-button type="primary" @click="onSearch">搜索</el-button>
            <el-button @click="resetFilters">重置</el-button>
          </div>
        </div>
      </template>

      <el-table :data="records" stripe style="width:100%" @row-click="openDetail" class="clickable-table">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="stockCode" label="股票代码" width="110">
          <template #default="{ row }">{{ row.stockCode || row.stock_code }}</template>
        </el-table-column>
        <el-table-column prop="stockName" label="股票名称" width="120">
          <template #default="{ row }">{{ row.stockName || row.stock_name }}</template>
        </el-table-column>
        <el-table-column label="情绪评分" width="120" align="center">
          <template #default="{ row }">
            <ScoreGauge :score="row.sentimentScore || row.sentiment_score || 0" :size="60" />
          </template>
        </el-table-column>
        <el-table-column label="决策类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="decisionTagType(row.decisionType || row.decision_type)" size="small">
              {{ decisionLabel(row.decisionType || row.decision_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作建议" min-width="160">
          <template #default="{ row }">{{ row.operationAdvice || row.operation_advice }}</template>
        </el-table-column>
        <el-table-column label="分析摘要" min-width="200">
          <template #default="{ row }">
            <span class="text-ellipsis">{{ row.analysisSummary || row.analysis_summary }}</span>
          </template>
        </el-table-column>
        <el-table-column label="风险提示" width="140">
          <template #default="{ row }">
            <span class="text-ellipsis">{{ row.riskWarning || row.risk_warning }}</span>
          </template>
        </el-table-column>
        <el-table-column label="分析时间" width="170">
          <template #default="{ row }">{{ row.createTime || row.create_time }}</template>
        </el-table-column>
      </el-table>

      <div style="display:flex;justify-content:center;margin-top:16px;gap:8px;align-items:center">
        <el-button :disabled="offset <= 0" @click="prevPage">上一页</el-button>
        <span style="font-size:13px;color:var(--el-text-color-secondary)">第 {{ currentPage }} 页</span>
        <el-button :disabled="records.length < pageSize" @click="nextPage">下一页</el-button>
      </div>

      <el-empty v-if="!records.length" description="暂无分析记录" />
    </el-card>

    <el-drawer v-model="drawerVisible" :title="'分析详情 - ' + (currentRecord.stockName || currentRecord.stock_name || '')"
      size="80%">
      <template v-if="currentRecord && currentRecord.id">
        <el-descriptions :column="2" border style="margin-bottom:16px" label-width="100px">
          <el-descriptions-item label="股票代码">{{ currentRecord.stockCode || currentRecord.stock_code
            }}</el-descriptions-item>
          <el-descriptions-item label="股票名称">{{ currentRecord.stockName || currentRecord.stock_name
            }}</el-descriptions-item>
          <el-descriptions-item label="情绪评分">
            <ScoreGauge :score="currentRecord.sentimentScore || currentRecord.sentiment_score || 0" :size="80" />
          </el-descriptions-item>
          <el-descriptions-item label="决策类型">
            <el-tag :type="decisionTagType(currentRecord.decisionType || currentRecord.decision_type)">
              {{ decisionLabel(currentRecord.decisionType || currentRecord.decision_type) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="操作建议" :span="2">{{ currentRecord.operationAdvice ||
            currentRecord.operation_advice }}</el-descriptions-item>
          <el-descriptions-item label="分析摘要" :span="2">{{ currentRecord.analysisSummary ||
            currentRecord.analysis_summary }}</el-descriptions-item>
          <el-descriptions-item label="风险提示" :span="2">{{ currentRecord.riskWarning || currentRecord.risk_warning
            }}</el-descriptions-item>
          <el-descriptions-item label="分析时间">{{ currentRecord.createTime || currentRecord.create_time
            }}</el-descriptions-item>
          <el-descriptions-item label="报告类型">{{ currentRecord.reportType || currentRecord.report_type || 'full'
            }}</el-descriptions-item>
        </el-descriptions>

        <el-divider content-position="left">完整报告</el-divider>
        <el-scrollbar v-if="reportMarkdown" class="report-content" max-height="calc(100vh - 20px)">
          <MarkdownRenderer :content="reportMarkdown" />
        </el-scrollbar>
        <el-empty v-else description="暂无完整报告内容" />
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { analysisApi } from '@/api/analysis'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import ScoreGauge from '@/components/common/ScoreGauge.vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'

const records = ref<any[]>([])
const filterCode = ref('')
const searchKeyword = ref('')
const offset = ref(0)
const pageSize = 20
const drawerVisible = ref(false)
const currentRecord = ref<Record<string, any>>({})
const reportMarkdown = ref('')

const currentPage = computed(() => Math.floor(offset.value / pageSize) + 1)

function decisionTagType(dt: string) {
  if (!dt) return 'info'
  const lower = dt.toLowerCase()
  if (lower.includes('buy') || lower.includes('买入')) return 'success'
  if (lower.includes('sell') || lower.includes('卖出')) return 'danger'
  if (lower.includes('hold') || lower.includes('持有')) return 'warning'
  return 'info'
}

function decisionLabel(dt: string) {
  if (!dt) return '-'
  const lower = dt.toLowerCase()
  if (lower.includes('buy') || lower.includes('买入')) return '买入'
  if (lower.includes('sell') || lower.includes('卖出')) return '卖出'
  if (lower.includes('hold') || lower.includes('持有')) return '持有'
  return dt
}

function onFilterStock(code: string) {
  filterCode.value = code
  searchKeyword.value = ''
  offset.value = 0
  loadRecords()
}

async function onSearch() {
  offset.value = 0
  await loadRecords()
}

function resetFilters() {
  filterCode.value = ''
  searchKeyword.value = ''
  offset.value = 0
  loadRecords()
}

async function loadRecords() {
  try {
    let res: any
    if (searchKeyword.value.trim()) {
      res = await analysisApi.historySearch(searchKeyword.value.trim(), pageSize)
    } else {
      res = await analysisApi.historyList({
        code: filterCode.value || undefined,
        limit: pageSize,
        offset: offset.value,
      })
    }
    records.value = Array.isArray(res) ? res : []
  } catch {
    ElMessage.error('加载分析历史失败')
  }
}

function prevPage() {
  offset.value = Math.max(0, offset.value - pageSize)
  loadRecords()
}

function nextPage() {
  offset.value += pageSize
  loadRecords()
}

async function openDetail(row: any) {
  currentRecord.value = row
  reportMarkdown.value = ''
  drawerVisible.value = true

  try {
    const res: any = await analysisApi.historyDetail(row.id)
    const detail = res || {}
    currentRecord.value = { ...row, ...detail }

    const reportJson = detail.reportJson || detail.report_json
    if (reportJson) {
      try {
        const parsed = typeof reportJson === 'string' ? JSON.parse(reportJson) : reportJson
        reportMarkdown.value = parsed.markdown || parsed.text || parsed.report || ''
      } catch {
        reportMarkdown.value = typeof reportJson === 'string' ? reportJson : JSON.stringify(reportJson, null, 2)
      }
    }
  } catch {
    ElMessage.error('加载详情失败')
  }
}

onMounted(() => {
  loadRecords()
})
</script>

<style scoped lang="scss">
.clickable-table {
  :deep(tbody tr) {
    cursor: pointer;
  }
}

.text-ellipsis {
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}

.report-content {
  padding: 0 4px;
}
</style>
