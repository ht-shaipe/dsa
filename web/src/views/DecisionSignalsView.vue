<template>
  <div class="decision-signals-view">
    <el-row :gutter="20" style="margin-bottom: 20px">
      <el-col :span="6" v-for="item in statsCards" :key="item.label">
        <el-card shadow="hover" class="stat-card">
          <el-statistic :title="item.label" :value="item.value" :precision="item.precision || 0" :suffix="item.suffix || ''" />
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:12px">
          <span>信号列表</span>
          <div style="display:flex;gap:12px;align-items:center;flex-wrap:wrap">
            <StockAutocomplete @select="onFilterStock" style="width:200px" />
            <el-select v-model="filterAction" placeholder="操作类型" clearable style="width:120px">
              <el-option label="买入" value="buy" />
              <el-option label="卖出" value="sell" />
              <el-option label="持有" value="hold" />
            </el-select>
            <el-select v-model="filterStatus" placeholder="状态" clearable style="width:120px">
              <el-option label="待处理" value="pending" />
              <el-option label="已采纳" value="accepted" />
              <el-option label="已拒绝" value="rejected" />
              <el-option label="已过期" value="expired" />
            </el-select>
            <el-button type="primary" @click="loadSignals">查询</el-button>
          </div>
        </div>
      </template>
      <el-row :gutter="16">
        <el-col :xs="24" :sm="12" :md="8" :lg="6" v-for="sig in signals" :key="sig.id">
          <el-card shadow="hover" class="signal-card" @click="openDetail(sig)">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
              <span class="signal-stock">{{ sig.code }} {{ sig.name }}</span>
              <el-tag :type="actionTagType(sig.action)" size="small">{{ actionLabel(sig.action) }}</el-tag>
            </div>
            <div style="display:flex;align-items:center;gap:12px">
              <ScoreGauge :score="sig.score || 0" :size="80" />
              <div>
                <div class="signal-field">置信度: {{ ((sig.confidence || 0) * 100).toFixed(0) }}%</div>
                <div class="signal-field">状态: <el-tag size="small" type="info">{{ statusLabel(sig.status) }}</el-tag></div>
              </div>
            </div>
            <div class="signal-reason">{{ sig.reason || sig.reasoning || '' }}</div>
          </el-card>
        </el-col>
      </el-row>
      <el-empty v-if="!signals.length" description="暂无信号" />
    </el-card>

    <el-drawer v-model="drawerVisible" :title="'信号详情 - ' + (currentSignal.code || '')" size="500px">
      <template v-if="currentSignal">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="股票代码">{{ currentSignal.code }}</el-descriptions-item>
          <el-descriptions-item label="股票名称">{{ currentSignal.name }}</el-descriptions-item>
          <el-descriptions-item label="操作">
            <el-tag :type="actionTagType(currentSignal.action)">{{ actionLabel(currentSignal.action) }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="评分">{{ currentSignal.score }}</el-descriptions-item>
          <el-descriptions-item label="置信度">{{ ((currentSignal.confidence || 0) * 100).toFixed(0) }}%</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag>{{ statusLabel(currentSignal.status) }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="理由">
            <MarkdownRenderer :content="currentSignal.reason || currentSignal.reasoning || ''" />
          </el-descriptions-item>
        </el-descriptions>

        <div style="margin-top: 20px" v-if="currentSignal.status === 'pending'">
          <el-divider content-position="left">操作</el-divider>
          <el-button type="success" @click="updateStatus('accepted')">采纳</el-button>
          <el-button type="danger" @click="updateStatus('rejected')">拒绝</el-button>
        </div>

        <div style="margin-top: 20px">
          <el-divider content-position="left">反馈</el-divider>
          <el-form :model="feedbackForm" label-width="60px">
            <el-form-item label="反馈">
              <el-input v-model="feedbackForm.feedback" type="textarea" :rows="3" placeholder="输入对该信号的反馈" />
            </el-form-item>
            <el-form-item label="评分">
              <el-rate v-model="feedbackForm.rating" :max="5" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="submitFeedback">提交反馈</el-button>
            </el-form-item>
          </el-form>
        </div>

        <div style="margin-top: 20px">
          <el-divider content-position="left">后续结果</el-divider>
          <el-table :data="outcomes" stripe size="small">
            <el-table-column prop="outcome" label="结果" width="80">
              <template #default="{ row }">
                <el-tag :type="row.outcome === 'hit' ? 'success' : 'danger'" size="small">
                  {{ row.outcome === 'hit' ? '命中' : '未中' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="priceChange" label="价格变化" width="100">
              <template #default="{ row }">{{ Number(row.priceChange || row.price_change || 0).toFixed(2) }}</template>
            </el-table-column>
            <el-table-column prop="evaluatedAt" label="评估时间" min-width="160">
              <template #default="{ row }">{{ row.evaluated_at || row.evaluated_at || '-' }}</template>
            </el-table-column>
          </el-table>
        </div>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { decisionApi } from '@/api/decision'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import ScoreGauge from '@/components/common/ScoreGauge.vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'

const signals = ref<any[]>([])
const stats = ref<Record<string, any>>({})
const outcomes = ref<any[]>([])
const drawerVisible = ref(false)
const currentSignal = ref<Record<string, any>>({})
const filterCode = ref('')
const filterAction = ref('')
const filterStatus = ref('')
const feedbackForm = ref({ feedback: '', rating: 3 })

const statsCards = computed(() => [
  { label: '胜率', value: Number(stats.value.winRate || stats.value.win_rate || 0) * 100, precision: 1, suffix: '%' },
  { label: '总信号数', value: stats.value.totalSignals || stats.value.total_signals || 0 },
  { label: '命中数', value: stats.value.hits || 0 },
  { label: '未中数', value: stats.value.misses || 0 },
])

function actionTagType(action: string) {
  return action === 'buy' ? 'success' : action === 'sell' ? 'danger' : 'warning'
}

function actionLabel(action: string) {
  return action === 'buy' ? '买入' : action === 'sell' ? '卖出' : action === 'hold' ? '持有' : action || '-'
}

function statusLabel(status: string) {
  const map: Record<string, string> = { pending: '待处理', accepted: '已采纳', rejected: '已拒绝', expired: '已过期' }
  return map[status] || status || '-'
}

function onFilterStock(code: string) {
  filterCode.value = code
  loadSignals()
}

async function loadSignals() {
  try {
    const params: Record<string, any> = {}
    if (filterCode.value) params.code = filterCode.value
    if (filterAction.value) params.action = filterAction.value
    if (filterStatus.value) params.status = filterStatus.value
    const res: any = await decisionApi.list(params)
    signals.value = res.data || []
  } catch { /* ignore */ }
}

async function loadStats() {
  try {
    const res: any = await decisionApi.stats(filterCode.value || undefined)
    stats.value = res.data || {}
  } catch { /* ignore */ }
}

async function openDetail(sig: Record<string, any>) {
  currentSignal.value = sig
  drawerVisible.value = true
  feedbackForm.value = { feedback: '', rating: 3 }
  try {
    const res: any = await decisionApi.detail(sig.id)
    currentSignal.value = res.data || sig
  } catch { /* ignore */ }
  loadOutcomes(sig.id)
}

async function loadOutcomes(signal_id: number) {
  try {
    const res: any = await decisionApi.outcomes({ signalId, limit: 20 })
    outcomes.value = res.data || []
  } catch { /* ignore */ }
}

async function updateStatus(status: string) {
  try {
    await decisionApi.updateStatus(currentSignal.value.id, status)
    ElMessage.success('状态已更新')
    currentSignal.value.status = status
    loadSignals()
  } catch {
    ElMessage.error('更新状态失败')
  }
}

async function submitFeedback() {
  try {
    await decisionApi.feedback(
      currentSignal.value.id,
      feedbackForm.value.feedback,
      feedbackForm.value.rating,
    )
    ElMessage.success('反馈已提交')
    feedbackForm.value = { feedback: '', rating: 3 }
  } catch {
    ElMessage.error('提交反馈失败')
  }
}

onMounted(() => {
  loadSignals()
  loadStats()
})
</script>

<style scoped lang="scss">
.stat-card {
  text-align: center;
}
.signal-card {
  margin-bottom: 16px;
  cursor: pointer;
  transition: transform 0.2s;
  &:hover {
    transform: translateY(-2px);
  }
}
.signal-stock {
  font-size: 16px;
  font-weight: 500;
}
.signal-field {
  font-size: 13px;
  color: var(--el-text-color-regular);
  margin-bottom: 4px;
}
.signal-reason {
  margin-top: 8px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
</style>
