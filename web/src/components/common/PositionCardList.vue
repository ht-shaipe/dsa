<template>
  <div class="position-card-list">
    <div v-if="!positions.length" class="position-empty">
      <el-empty :description="emptyText" :image-size="60" />
    </div>
    <div v-else class="position-grid" :style="gridStyle">
      <div v-for="row in localPositions" :key="row.stockCode || row.code" class="position-card"
        :class="{ 'pnl-up-border': isUp(row), 'pnl-down-border': isDown(row) }" @click="$emit('click', row)">
        <div class="position-card-header">
          <div class="position-card-title">
            <span class="position-card-name">{{ row.stockName || row.name || '-' }}</span>
            <span class="position-card-code">{{ row.stockCode || row.code }}</span>
          </div>
          <span class="position-card-mv">{{ formatMoney(row.marketValue) }}</span>
        </div>

        <div class="position-card-price-row">
          <span :class="['position-card-current', priceDir(row)]">
            {{ formatNum(row.currentPrice, 2) }}
          </span>
          <span v-if="row.todayChangePercent !== undefined" :class="['position-card-today-pnl', todayPnlDirClass(row)]">
            {{ row.todayChangePercent >= 0 ? '+' : '' }}{{ formatNum(row.todayChangePercent, 2) }}%
          </span>
          <span v-if="row.todayPnl !== undefined" :class="['position-card-today-pnl', todayPnlDirClass(row)]">
            {{ row.todayPnl >= 0 ? '+' : '' }}{{ formatNum(row.todayPnl, 2) }}
          </span>
        </div>

        <div class="position-card-details">
          <div class="detail-item">
            <span class="detail-label">成本</span>
            <span class="detail-value">{{ formatNum(row.avgCost, 3) }}</span>
          </div>

          <div class="detail-item">
            <span class="detail-label">持仓</span>
            <span class="detail-value">{{ row.quantity }}股</span>
          </div>

        </div>

        <div class="position-card-footer">
          <div class="pnl-row">
            <span class="pnl-item">
              <span class="footer-label">盈亏</span>
              <span :class="['footer-pnl', pnlDirClass(row)]">{{ pnlText(row.unrealizedPnl) }}</span>
            </span>
            <span class="pnl-item">
              <span class="footer-label">比例</span>
              <span :class="['footer-pnl', pnlDirClass(row)]">{{ pnlPercentText(row) }}</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { stockApi } from '@/api/stock'
import { useTradingInterval } from '@/composables/useTradingInterval'
import { formatNum, formatMoney, pnlText, pnlDir } from '@/utils/format'

export interface PositionItem {
  stockCode?: string
  code?: string
  stockName?: string
  name?: string
  quantity?: number
  avgCost?: number
  currentPrice?: number
  marketValue?: number
  unrealizedPnl?: number
  [key: string]: any
}

const props = withDefaults(defineProps<{
  positions: PositionItem[]
  columns?: number
  emptyText?: string
  showPnlBadge?: boolean
  refreshInterval?: number
  autoRefresh?: boolean
}>(), {
  columns: 4,
  emptyText: '暂无持仓',
  showPnlBadge: true,
  refreshInterval: 10000,
  autoRefresh: true,
})

const emit = defineEmits<{
  click: [row: PositionItem]
  quotesUpdated: [data: PositionItem[]]
}>()

// 本地数据，在 props 基础上叠加实时行情
const localPositions = ref<PositionItem[]>([])

watch(() => props.positions, (val) => {
  localPositions.value = val.map(p => ({ ...p }))
}, { immediate: true, deep: true })

async function refreshQuotes() {
  if (!localPositions.value.length) return
  const codes = localPositions.value
    .map(s => (s.stockCode || s.code || '').replace(/^(sh|sz|bj)/, ''))
    .filter(Boolean)
    .join(',')
  if (!codes) return
  try {
    const res: any = await stockApi.quotes(codes)
    const list = Array.isArray(res) ? res : []
    const map = new Map<string, any>()
    for (const q of list) {
      const qCode = (q.code || q.symbol || '').replace(/^(sh|sz|bj)/, '')
      map.set(qCode, q)
    }
    let changed = false
    localPositions.value = localPositions.value.map(s => {
      const code = (s.stockCode || s.code || '').replace(/^(sh|sz|bj)/, '')
      const q = map.get(code)
      if (!q) {
        // 如果没有找到行情数据，保留原有数据（包括今日盈亏信息）
        return s
      }
      changed = true
      const price = q.price ?? q.close
      const qty = Number(s.quantity || 0)
      const cost = Number(s.avgCost || 0)
      const mv = price != null ? price * qty : s.marketValue
      const pnl = price != null ? (price - cost) * qty : s.unrealizedPnl

      // 尝试多个可能的字段名来获取涨跌幅和涨跌额
      let changePercent = q.changePercent ?? q.percent ?? q.chg ?? q.change_percent ?? 0
      let change = q.change ?? q.diff ?? q.priceChange ?? 0

      // 如果没有涨跌额，用当前价和昨收价计算
      if ((!change || change === 0) && price != null) {
        const preClose = q.preClose ?? q.prevClose ?? q.previousClose ?? 0
        if (preClose > 0) {
          change = price - preClose
          if (!changePercent) {
            changePercent = (change / preClose) * 100
          }
        } else if (changePercent !== 0) {
          change = price - price / (1 + changePercent / 100)
        }
      }

      // 计算今天盈亏：涨跌额 × 持仓数量
      const todayPnl = change * qty

      // 确保值不为 null，如果是 0 也要保留
      const finalChangePercent = (changePercent !== undefined && changePercent !== null) ? changePercent : 0
      const finalTodayPnl = (todayPnl !== undefined && todayPnl !== null) ? todayPnl : 0

      return {
        ...s,
        currentPrice: price ?? s.currentPrice,
        marketValue: mv,
        unrealizedPnl: pnl,
        todayChangePercent: finalChangePercent,
        todayPnl: finalTodayPnl,
      }
    })
    if (changed) {
      emit('quotesUpdated', localPositions.value)
    }
  } catch { /* ignore */ }
}

const quoteTimer = useTradingInterval(refreshQuotes, props.refreshInterval)

onMounted(() => {
  // 立即刷新一次行情，获取当日涨跌幅数据
  refreshQuotes()
  if (props.autoRefresh) {
    quoteTimer.start()
  }
})

onBeforeUnmount(() => {
  quoteTimer.stop()
})

const gridStyle = computed(() => ({
  gridTemplateColumns: `repeat(${props.columns}, 1fr)`,
}))

function isUp(row: PositionItem) {
  return Number(row.unrealizedPnl || 0) > 0
}

function isDown(row: PositionItem) {
  return Number(row.unrealizedPnl || 0) < 0
}

function pnlDirClass(row: PositionItem) {
  return pnlDir(row.unrealizedPnl)
}

function priceDir(row: PositionItem) {
  const price = Number(row.currentPrice || 0)
  const cost = Number(row.avgCost || 0)
  return price >= cost ? 'up' : 'down'
}

function todayPnlDirClass(row: PositionItem) {
  if (row.todayChangePercent == null) return ''
  return Number(row.todayChangePercent) >= 0 ? 'up' : 'down'
}

function pnlPercentText(row: PositionItem) {
  const cost = Number(row.avgCost || 0)
  const price = Number(row.currentPrice || 0)
  if (!cost) return '0.00%'
  const pct = ((price - cost) / cost) * 100
  return (pct >= 0 ? '+' : '') + pct.toFixed(2) + '%'
}
</script>

<style scoped lang="scss">
.position-card-list {
  width: 100%;
}

.position-empty {
  padding: 20px 0;
}

.position-grid {
  display: grid;
  gap: 8px;
}

.position-card {
  padding: 14px 18px;
  border-radius: 8px;
  border: 1px solid var(--el-border-color-lighter);
  background: var(--el-bg-color);
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
  min-width: 0;

  &::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: transparent;
    border-radius: 0 2px 2px 0;
    transition: background 0.2s;
  }

  &:hover {
    border-color: var(--el-color-primary-light-5);
    background: var(--el-color-primary-light-9);
  }

  &.pnl-up-border::before {
    background: #f56c6c;
  }

  &.pnl-down-border::before {
    background: #67c23a;
  }
}

.position-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.position-card-title {
  display: flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}

.position-card-name {
  font-size: 15px;
  font-weight: 600;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.position-card-code {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.position-pnl-badge {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #f56c6c;
    background: rgba(245, 108, 108, 0.1);
  }

  &.down {
    color: #67c23a;
    background: rgba(103, 194, 58, 0.1);
  }
}

.position-card-price-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 10px;
}

.position-card-current {
  font-size: 24px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #f56c6c;
  }

  &.down {
    color: #67c23a;
  }
}

.position-card-mv {
  font-size: 16px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-primary);
}

.position-card-today-pnl {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #f56c6c;
  }

  &.down {
    color: #67c23a;
  }
}

.position-card-details {
  display: flex;
  gap: 16px;
  margin-bottom: 10px;
  justify-content: space-between;
}

.detail-item {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.detail-label {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
}

.detail-value {
  font-size: 13px;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #f56c6c;
  }

  &.down {
    color: #67c23a;
  }
}

.position-card-footer {
  padding-top: 10px;
  border-top: 1px solid var(--el-border-color-extra-light);
}

.pnl-row {
  display: flex;
  gap: 20px;
  align-items: baseline;
}

.pnl-item {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.footer-label {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
}

.footer-pnl {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #f56c6c;
  }

  &.down {
    color: #67c23a;
  }
}
</style>
