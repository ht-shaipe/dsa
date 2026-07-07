<template>
  <div class="score-gauge">
    <el-progress
      type="dashboard"
      :percentage="score"
      :color="color"
      :width="size"
      :stroke-width="6"
    >
      <template #default>
        <div class="gauge-inner">
          <span class="gauge-score">{{ score }}</span>
          <span class="gauge-label">{{ label }}</span>
        </div>
      </template>
    </el-progress>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  score: number
  size?: number
}>()

const label = computed(() => {
  if (props.score >= 80) return '看多'
  if (props.score >= 60) return '偏多'
  if (props.score >= 40) return '中性'
  if (props.score >= 20) return '偏空'
  return '看空'
})

const color = computed(() => {
  if (props.score >= 80) return '#67c23a'
  if (props.score >= 60) return '#95d475'
  if (props.score >= 40) return '#e6a23c'
  if (props.score >= 20) return '#f56c6c'
  return '#c45656'
})
</script>

<style scoped lang="scss">
.score-gauge {
  display: inline-flex;
}
.gauge-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
}
.gauge-score {
  font-size: 22px;
  font-weight: bold;
}
.gauge-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
