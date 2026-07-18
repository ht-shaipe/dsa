<template>
  <div class="kline-view">
    <div class="kline-header">
      <el-button :icon="ArrowLeft" @click="goBack" text>返回</el-button>
      <span class="kline-title">{{ stockName || code }}（{{ code }}）</span>
      <span v-if="quote" class="kline-quote" :class="pnlClass">
        {{ formatPrice(quote.close || quote.price) }}
        <template v-if="isValidNumber(quote.changePercent)">
          &nbsp;{{ Number(quote.changePercent) >= 0 ? '+' : '' }}{{ Number(quote.changePercent).toFixed(2) }}%
        </template>
      </span>
    </div>

    <div v-if="klineLoading" class="kline-loading">
      <el-icon class="is-loading" :size="28"><Loading /></el-icon>
      <span style="margin-left:8px;color:var(--el-text-color-secondary)">加载K线数据...</span>
    </div>

    <div v-else-if="klineError" class="kline-loading">
      <el-result icon="warning" :title="klineError" sub-title="请检查股票代码或稍后重试">
        <template #extra>
          <el-button type="primary" @click="loadKline">重新加载</el-button>
          <el-button @click="goBack">返回</el-button>
        </template>
      </el-result>
    </div>

    <div v-else-if="klineData.length" class="kline-chart-container">
      <KlineChart
        :data="klineData"
        :height="chartHeight"
        :show-m-a="true"
        :show-boll="true"
        :show-volume="true"
        :show-m-a-c-d="true"
      />
    </div>

    <el-empty v-else description="暂无K线数据" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Loading } from '@element-plus/icons-vue'
import KlineChart from '@/components/common/KlineChart.vue'
import { stockApi } from '@/api/stock'

const route = useRoute()
const router = useRouter()

const code = ref((route.params.code as string) || (route.query.code as string) || '')
const stockName = ref((route.query.name as string) || '')
const klineData = ref<any[]>([])
const klineLoading = ref(false)
const klineError = ref('')
const quote = ref<Record<string, any> | null>(null)

const chartHeight = computed(() => {
  const height = window.innerHeight - 160
  return height > 0 ? Math.max(400, height) : 500
})

const pnlClass = computed(() => {
  if (!quote.value || !isValidNumber(quote.value.changePercent)) return ''
  return Number(quote.value.changePercent) >= 0 ? 'pnl-up' : 'pnl-down'
})

function formatPrice(value: any): string {
  const num = Number(value)
  return isNaN(num) ? '-' : num.toFixed(2)
}

function isValidNumber(value: any): boolean {
  if (value === null || value === undefined) return false
  const num = Number(value)
  return !isNaN(num)
}

function goBack() {
  if (window.history.length > 1) {
    router.back()
  } else {
    router.push('/watchlist')
  }
}

async function loadKline() {
  if (!code.value) {
    klineError.value = '股票代码为空'
    klineData.value = []
    return
  }
  klineLoading.value = true
  klineError.value = ''
  try {
    const res: any = await stockApi.kline({ code: code.value, period: 'daily' })
    const data = Array.isArray(res) ? res : (res?.data && Array.isArray(res.data) ? res.data : [])
    if (!data.length) {
      klineError.value = '暂无K线数据'
    }
    klineData.value = data
  } catch (e: any) {
    klineData.value = []
    klineError.value = e?.message || 'K线数据加载失败'
  } finally {
    klineLoading.value = false
  }
}

async function loadQuote() {
  if (!code.value) return
  try {
    const res: any = await stockApi.quote(code.value)
    quote.value = res || null
  } catch {
    quote.value = null
  }
}

let quoteTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  loadKline()
  loadQuote()
  quoteTimer = setInterval(loadQuote, 10000)
})

onBeforeUnmount(() => {
  if (quoteTimer) { clearInterval(quoteTimer); quoteTimer = null }
})
</script>

<style scoped lang="scss">
.kline-view {
  position: relative;
  min-height: calc(100vh - 160px);
}
.kline-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.kline-title {
  font-size: 16px;
  font-weight: 600;
}
.kline-quote {
  font-size: 18px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.kline-loading {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 400px;
}
.kline-chart-container {
  min-height: 400px;
  position: relative;
}
.pnl-up { color: var(--el-color-danger); }
.pnl-down { color: var(--el-color-success); }
</style>
