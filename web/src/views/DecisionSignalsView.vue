<template>
  <div class="decision-signals-view">
    <el-alert type="info" :closable="false" style="margin-bottom: 20px">
      <template #title>
        <span style="font-weight:600">决策信号使用指南</span>
      </template>
      <div style="line-height:2">
        <p><b>1. 生成信号：</b>前往「智能分析」页面对股票执行 AI 分析，分析完成后点击下方「提取信号」按钮，系统会自动从分析报告中提取买卖/持有信号。</p>
        <p><b>2. 查看与决策：</b>信号以卡片形式展示在下方，点击卡片可查看详情。对「待处理」信号可以采纳（标记为已过期/跟进）或拒绝（标记为已失效）。</p>
        <p><b>3. 回测验证：</b>点击「回测验证」可跳转到回测页面，用历史数据验证信号的可靠性。</p>
        <p><b>4. 反馈优化：</b>在信号详情中提交文字反馈和评分，帮助系统持续优化。</p>
      </div>
    </el-alert>

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
            <StockAutocomplete v-model="filterSearch" @select="onFilterStock" style="width:200px" />
            <el-select v-model="filterAction" placeholder="操作类型" clearable style="width:120px">
              <el-option label="买入" value="buy" />
              <el-option label="卖出" value="sell" />
              <el-option label="持有" value="hold" />
            </el-select>
            <el-select v-model="filterStatus" placeholder="状态" clearable style="width:120px">
              <el-option label="待处理" value="active" />
              <el-option label="已过期" value="expired" />
              <el-option label="已失效" value="invalidated" />
              <el-option label="已达标" value="closed" />
              <el-option label="已归档" value="archived" />
            </el-select>
            <el-button type="primary" @click="loadSignals">查询</el-button>
            <el-button type="warning" :loading="extracting" @click="extractBatch">
              提取信号
            </el-button>
            <el-button :loading="reassessing" @click="reassessSignals">
              重新评估
            </el-button>
          </div>
        </div>
      </template>

      <el-row :gutter="16">
        <el-col :xs="24" :sm="12" :md="8" :lg="6" v-for="sig in signals" :key="sig.id">
          <el-card shadow="hover" class="signal-card" @click="openDetail(sig)">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
              <span class="signal-stock">{{ sig.stockCode || sig.code }} {{ sig.stockName || sig.name }}</span>
              <el-tag :type="actionTagType(sig.action)" size="small">{{ actionLabel(sig.action) }}</el-tag>
            </div>
            <div style="display:flex;align-items:center;gap:12px">
              <ScoreGauge :score="sig.sentimentScore || sig.score || 0" :size="80" />
              <div>
                <div class="signal-field">置信度: {{ sig.confidenceLevel || '-' }}</div>
                <div class="signal-field">状态: <el-tag size="small" type="info">{{ statusLabel(sig.status) }}</el-tag></div>
              </div>
            </div>
            <div class="signal-reason">{{ sig.reasoning || sig.reason || sig.evidence || '' }}</div>
          </el-card>
        </el-col>
      </el-row>

      <el-empty v-if="!signals.length" description="暂无信号，请先在「智能分析」页面分析股票，然后点击「提取信号」" />
    </el-card>

    <el-drawer v-model="drawerVisible" :title="'信号详情 - ' + (currentSignal.stockCode || currentSignal.code || '')" size="500px">
      <template v-if="currentSignal && currentSignal.id">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="股票代码">{{ currentSignal.stockCode || currentSignal.code }}</el-descriptions-item>
          <el-descriptions-item label="股票名称">{{ currentSignal.stockName || currentSignal.name }}</el-descriptions-item>
          <el-descriptions-item label="操作">
            <el-tag :type="actionTagType(currentSignal.action)">{{ actionLabel(currentSignal.action) }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="情绪评分">
            <ScoreGauge :score="currentSignal.sentimentScore || currentSignal.score || 0" :size="60" />
          </el-descriptions-item>
          <el-descriptions-item label="置信度">{{ currentSignal.confidenceLevel || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag>{{ statusLabel(currentSignal.status) }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="入场价">{{ currentSignal.entryPrice || '-' }}</el-descriptions-item>
          <el-descriptions-item label="目标价">{{ currentSignal.targetPrice || '-' }}</el-descriptions-item>
          <el-descriptions-item label="止损价">{{ currentSignal.stopLoss || '-' }}</el-descriptions-item>
          <el-descriptions-item label="信号日期">{{ currentSignal.signalDate || '-' }}</el-descriptions-item>
          <el-descriptions-item label="理由" :span="2">
            <MarkdownRenderer :content="currentSignal.reasoning || currentSignal.reason || ''" />
          </el-descriptions-item>
          <el-descriptions-item v-if="currentSignal.evidence" label="依据" :span="2">
            <MarkdownRenderer :content="currentSignal.evidence" />
          </el-descriptions-item>
        </el-descriptions>

        <div style="margin-top: 20px" v-if="statusNum(currentSignal.status) === 1">
          <el-divider content-position="left">操作</el-divider>
          <el-button type="success" @click="updateStatus('closed')">采纳</el-button>
          <el-button type="danger" @click="updateStatus('invalidated')">拒绝</el-button>
          <el-button type="primary" @click="goBacktest">回测验证</el-button>
        </div>

        <div style="margin-top: 12px" v-else>
          <el-divider content-position="left">操作</el-divider>
          <el-button type="primary" @click="goBacktest">回测验证</el-button>
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
            <el-table-column label="方向正确" width="80">
              <template #default="{ row }">
                <el-tag :type="row.directionCorrect ? 'success' : 'danger'" size="small">
                  {{ row.directionCorrect ? '正确' : '错误' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="实际收益" width="100">
              <template #default="{ row }">{{ Number(row.actualReturn || row.actual_return || 0).toFixed(2) }}%</template>
            </el-table-column>
            <el-table-column label="最大回撤" width="100">
              <template #default="{ row }">{{ Number(row.maxDrawdown || row.max_drawdown || 0).toFixed(2) }}%</template>
            </el-table-column>
            <el-table-column label="评估时间" min-width="160">
              <template #default="{ row }">{{ row.evalDate || row.eval_date || '-' }}</template>
            </el-table-column>
          </el-table>
        </div>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { decisionApi } from '@/api/decision'
import StockAutocomplete from '@/components/common/StockAutocomplete.vue'
import ScoreGauge from '@/components/common/ScoreGauge.vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'

const router = useRouter()
const signals = ref<any[]>([])
const stats = ref<Record<string, any>>({})
const outcomes = ref<any[]>([])
const drawerVisible = ref(false)
const currentSignal = ref<Record<string, any>>({})
const filterCode = ref('')
const filterSearch = ref('')
const filterAction = ref('')
const filterStatus = ref('')
const feedbackForm = ref({ feedback: '', rating: 3 })
const extracting = ref(false)
const reassessing = ref(false)

const statsCards = computed(() => [
  { label: '看多信号', value: Number(stats.value.bullish || 0), precision: 0, suffix: '' },
  { label: '看空信号', value: Number(stats.value.bearish || 0), precision: 0, suffix: '' },
  { label: '中性信号', value: Number(stats.value.neutral || 0), precision: 0, suffix: '' },
  { label: '总信号数', value: Number(stats.value.total || 0), precision: 0, suffix: '' },
])

function actionTagType(action: string) {
  return action === 'buy' || action === 'add' ? 'success' : action === 'sell' || action === 'reduce' ? 'danger' : 'warning'
}

function actionLabel(action: string) {
  const map: Record<string, string> = { buy: '买入', add: '加仓', sell: '卖出', reduce: '减仓', hold: '持有', avoid: '回避' }
  return map[action] || action || '-'
}

function statusLabel(status: any): string {
  const n = statusNum(status)
  const map: Record<number, string> = { 1: '待处理', 2: '已过期', 3: '已失效', 4: '已达标', 5: '已归档' }
  return map[n] || String(status) || '-'
}

function statusNum(status: any): number {
  if (typeof status === 'number') return status
  const map: Record<string, number> = { active: 1, pending: 1, expired: 2, invalidated: 3, closed: 4, archived: 5 }
  return map[String(status)] || 0
}

function onFilterStock(code: string) {
  filterCode.value = code
  filterSearch.value = code
  loadSignals()
}

async function loadSignals() {
  try {
    const params: Record<string, any> = {}
    if (filterCode.value) params.code = filterCode.value
    if (filterAction.value) params.action = filterAction.value
    if (filterStatus.value) params.status = filterStatus.value
    const res: any = await decisionApi.list(params)
    signals.value = Array.isArray(res) ? res : []
  } catch { /* ignore */ }
}

async function loadStats() {
  try {
    const res: any = await decisionApi.stats(filterCode.value || undefined)
    stats.value = res || {}
  } catch { /* ignore */ }
}

async function extractBatch() {
  extracting.value = true
  try {
    const res: any = await decisionApi.extractBatch(20)
    const count = res?.extractedCount || res?.extracted_count || 0
    const errors = res?.errors || []
    if (count > 0) {
      ElMessage.success(`成功提取 ${count} 条信号${errors.length ? `，${errors.length} 条失败` : ''}`)
    } else {
      ElMessage.info('没有新的分析报告需要提取信号，请先在「智能分析」页面分析股票')
    }
    loadSignals()
    loadStats()
  } catch (e: any) {
    ElMessage.error('提取信号失败: ' + (e.message || '未知错误'))
  } finally {
    extracting.value = false
  }
}

async function reassessSignals() {
  reassessing.value = true
  try {
    await decisionApi.reassess()
    ElMessage.success('重新评估完成')
    loadSignals()
    loadStats()
  } catch (e: any) {
    ElMessage.error('重新评估失败: ' + (e.message || '未知错误'))
  } finally {
    reassessing.value = false
  }
}

async function openDetail(sig: Record<string, any>) {
  currentSignal.value = sig
  drawerVisible.value = true
  feedbackForm.value = { feedback: '', rating: 3 }
  try {
    const res: any = await decisionApi.detail(sig.id)
    currentSignal.value = res || sig
  } catch { /* ignore */ }
  loadOutcomes(sig.id)
}

async function loadOutcomes(signalId: number) {
  try {
    const res: any = await decisionApi.outcomes({ signalId, limit: 20 })
    outcomes.value = Array.isArray(res) ? res : []
  } catch { /* ignore */ }
}

async function updateStatus(status: string) {
  try {
    await decisionApi.updateStatus(currentSignal.value.id, status)
    ElMessage.success('状态已更新')
    currentSignal.value.status = statusNum(status)
    loadSignals()
  } catch {
    ElMessage.error('更新状态失败')
  }
}

function goBacktest() {
  const code = currentSignal.value.stockCode || currentSignal.value.code || ''
  router.push({ path: '/backtest', query: { code } })
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
