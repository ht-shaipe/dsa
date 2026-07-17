<template>
  <div class="quick-start-card">
    <div class="qs-icon">
      <el-icon :size="40" color="var(--el-color-primary)"><Promotion /></el-icon>
    </div>
    <div class="qs-content">
      <h2 class="qs-title">欢迎使用 DSA</h2>
      <p class="qs-desc">系统检测到股票池为空，需要初始化数据后才能使用分析、筛选等功能。</p>
      <div class="qs-steps">
        <div class="qs-step" :class="{ active: currentPhase === 'init_pool' || currentPhase === 'init_pool_writing', done: poolDone }">
          <span class="qs-step-num">{{ poolDone ? '✓' : '①' }}</span>
          <span class="qs-step-text">初始化股票池</span>
          <span v-if="poolDone" class="qs-step-detail">（{{ poolCount }} 只）</span>
        </div>
        <div class="qs-step" :class="{ active: currentPhase === 'init_daily_data' || currentPhase === 'calculating_indicators', done: dailyDone }">
          <span class="qs-step-num">{{ dailyDone ? '✓' : '②' }}</span>
          <span class="qs-step-text">同步日线数据</span>
          <span v-if="taskRunning && currentPhase === 'init_daily_data'" class="qs-step-detail">
            （{{ taskDone.toLocaleString() }}/{{ taskTotal.toLocaleString() }}）
          </span>
        </div>
      </div>
      <div class="qs-actions">
        <el-button
          v-if="!taskRunning && !poolDone"
          type="primary"
          size="large"
          @click="$emit('start')"
        >
          <el-icon><CaretRight /></el-icon>
          一键初始化
        </el-button>
        <div v-if="taskRunning" class="qs-progress">
          <el-icon class="is-loading" :size="16"><Loading /></el-icon>
          <span>{{ phaseLabel }}</span>
          <template v-if="taskTotal > 0">
            <span class="qs-pct">{{ (taskDone / taskTotal * 100).toFixed(0) }}%</span>
          </template>
        </div>
        <el-button
          v-if="!taskRunning && poolDone && !dailyDone"
          type="primary"
          size="large"
          @click="$emit('start')"
        >
          <el-icon><CaretRight /></el-icon>
          继续同步日线数据
        </el-button>
        <el-button
          v-if="poolDone && dailyDone"
          type="success"
          size="large"
          @click="$emit('done')"
        >
          开始使用 →
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Promotion, CaretRight, Loading } from '@element-plus/icons-vue'
import { useTaskStore, getPhaseLabel } from '@/stores/task'

const props = defineProps<{
  poolCount: number
  dailyCount: number
}>()

defineEmits<{
  start: []
  done: []
}>()

const taskStore = useTaskStore()

const task = computed(() => taskStore.tasks['quick_init'] || taskStore.tasks['init_stock_pool'] || taskStore.tasks['init_daily_data'] || {})
const taskRunning = computed(() => !!task.value.running)
const taskTotal = computed(() => task.value.total || 0)
const taskDone = computed(() => task.value.done || 0)
const currentPhase = computed(() => task.value.phase || '')

const poolDone = computed(() => props.poolCount > 0)
const dailyDone = computed(() => props.dailyCount > 0)

const phaseLabel = computed(() => {
  if (!taskRunning.value) return ''
  return getPhaseLabel(currentPhase.value)
})
</script>

<style scoped lang="scss">
.quick-start-card {
  display: flex;
  gap: 24px;
  padding: 32px;
  background: var(--el-bg-color);
  border-radius: 12px;
  border: 2px dashed var(--el-color-primary-light-5);
  margin-bottom: 20px;
}

.qs-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--el-color-primary-light-9);
}

.qs-content {
  flex: 1;
  min-width: 0;
}

.qs-title {
  margin: 0 0 8px;
  font-size: 22px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.qs-desc {
  margin: 0 0 16px;
  font-size: 14px;
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}

.qs-steps {
  display: flex;
  gap: 32px;
  margin-bottom: 20px;
}

.qs-step {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  color: var(--el-text-color-placeholder);
  transition: color 0.3s;

  &.active {
    color: var(--el-color-primary);
    font-weight: 600;
  }

  &.done {
    color: var(--el-color-success);
  }
}

.qs-step-num {
  font-size: 16px;
}

.qs-step-text {
  font-size: 14px;
}

.qs-step-detail {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.qs-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}

.qs-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--el-color-primary);
}

.qs-pct {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
</style>
