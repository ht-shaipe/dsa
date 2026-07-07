<template>
  <div ref="chartRef" :style="{ width: '100%', height: height + 'px' }" />
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import * as echarts from 'echarts'

const props = defineProps<{
  data: any[]
  height?: number
}>()

const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null

function renderChart() {
  if (!chartRef.value || !props.data?.length) return

  if (!chart) {
    chart = echarts.init(chartRef.value)
  }

  const dates = props.data.map((d: any) => d.date || d.日期 || '')
  const opens = props.data.map((d: any) => d.open || d.开盘 || 0)
  const closes = props.data.map((d: any) => d.close || d.收盘 || 0)
  const lows = props.data.map((d: any) => d.low || d.最低 || 0)
  const highs = props.data.map((d: any) => d.high || d.最高 || 0)
  const volumes = props.data.map((d: any) => d.volume || d.成交量 || 0)

  chart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
    legend: { data: ['K线', '成交量'] },
    grid: [
      { left: '10%', right: '8%', top: '5%', height: '55%' },
      { left: '10%', right: '8%', top: '68%', height: '20%' },
    ],
    xAxis: [
      { type: 'category', data: dates, gridIndex: 0, boundaryGap: true },
      { type: 'category', data: dates, gridIndex: 1, boundaryGap: true },
    ],
    yAxis: [
      { type: 'value', gridIndex: 0, scale: true },
      { type: 'value', gridIndex: 1, scale: true },
    ],
    series: [
      {
        name: 'K线',
        type: 'candlestick',
        data: props.data.map((_: any, i: number) => [opens[i], closes[i], lows[i], highs[i]]),
        xAxisIndex: 0,
        yAxisIndex: 0,
        itemStyle: {
          color: '#ef232a',
          color0: '#14b143',
          borderColor: '#ef232a',
          borderColor0: '#14b143',
        },
      },
      {
        name: '成交量',
        type: 'bar',
        data: volumes,
        xAxisIndex: 1,
        yAxisIndex: 1,
        itemStyle: {
          color: (params: any) => {
            return closes[params.dataIndex] >= opens[params.dataIndex] ? '#14b143' : '#ef232a'
          },
        },
      },
    ],
  })
}

onMounted(() => {
  renderChart()
  window.addEventListener('resize', () => chart?.resize())
})

watch(() => props.data, () => renderChart(), { deep: true })
</script>
