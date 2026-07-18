<template>
  <div ref="chartRef" :style="{ width: '100%', height: height + 'px', minHeight: '400px' }" />
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import * as echarts from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { CandlestickChart, LineChart, BarChart } from 'echarts/charts'
import { TooltipComponent, GridComponent, DataZoomComponent } from 'echarts/components'
import { formatMoney } from '@/utils/format'

echarts.use([CanvasRenderer, CandlestickChart, LineChart, BarChart, TooltipComponent, GridComponent, DataZoomComponent])

const props = withDefaults(defineProps<{
  data: any[]
  height?: number
  showMA?: boolean
  showBoll?: boolean
  showVolume?: boolean
  showMACD?: boolean
}>(), {
  height: 500,
  showMA: true,
  showBoll: true,
  showVolume: true,
  showMACD: true,
})

const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null

const colors = {
  red: '#ef232a',
  green: '#14b143',
  label: '#999',
  crosshair: '#409eff',
  ma5: '#ffd700',
  ma10: '#ff8c00',
  ma20: '#00bfff',
  ma30: '#96ceb4',
  ma60: '#ff69b4',
  bollUp: '#ef232a',
  bollDown: '#14b143',
  bollMid: '#409eff',
  dif: '#409eff',
  dea: '#ff6b6b',
  macdUp: '#ef232a',
  macdDown: '#14b143',
}

const parsedData = computed(() => {
  if (!props.data?.length) return { times: [], kline: [], vols: [], amounts: [], closes: [] }
  const times: string[] = []
  const kline: number[][] = []
  const vols: number[] = []
  const amounts: number[] = []
  const closes: number[] = []
  for (const d of props.data) {
    const date = d.date || d.日期 || ''
    const open = d.open || d.开盘 || 0
    const close = d.close || d.收盘 || 0
    const high = d.high || d.最高 || 0
    const low = d.low || d.最低 || 0
    const volume = d.volume || d.成交量 || 0
    const amount = d.amount || d.成交额 || 0
    times.push(date)
    kline.push([open, close, low, high])
    vols.push(volume)
    amounts.push(amount)
    closes.push(close)
  }
  return { times, kline, vols, amounts, closes }
})

function calculateMA(dayCount: number): (number | null)[] {
  const closes = parsedData.value.closes
  const result: (number | null)[] = []
  for (let i = 0; i < closes.length; i++) {
    if (i < dayCount - 1) { result.push(null); continue }
    let sum = 0
    let valid = 0
    for (let j = 0; j < dayCount; j++) {
      const v = closes[i - j]
      if (v != null && !isNaN(v)) { sum += v; valid++ }
    }
    result.push(valid === dayCount ? Number((sum / dayCount).toFixed(2)) : null)
  }
  return result
}

function calculateBoll(period: number = 20) {
  const closes = parsedData.value.closes
  const upper: (number | null)[] = []
  const lower: (number | null)[] = []
  for (let i = 0; i < closes.length; i++) {
    if (i < period - 1) { upper.push(null); lower.push(null); continue }
    let sum = 0
    let valid = 0
    for (let j = 0; j < period; j++) {
      const v = closes[i - j]
      if (v != null && !isNaN(v)) { sum += v; valid++ }
    }
    if (valid < period) { upper.push(null); lower.push(null); continue }
    const ma = sum / period
    let variance = 0
    for (let j = 0; j < period; j++) {
      const v = closes[i - j]
      if (v != null && !isNaN(v)) variance += (v - ma) ** 2
    }
    const stdDev = Math.sqrt(variance / period)
    upper.push(Number((ma + 2 * stdDev).toFixed(2)))
    lower.push(Number((ma - 2 * stdDev).toFixed(2)))
  }
  return { upper, lower, mid: calculateMA(period) }
}

function calculateEMA(data: number[], period: number): (number | null)[] {
  const result: (number | null)[] = []
  const k = 2 / (period + 1)
  let ema: number | null = null
  for (let i = 0; i < data.length; i++) {
    const v = data[i]
    if (v == null || isNaN(v)) { result.push(null); continue }
    if (ema === null) {
      ema = v
    } else {
      ema = v * k + ema * (1 - k)
    }
    result.push(Number(ema.toFixed(4)))
  }
  return result
}

function calculateMACD(fast: number = 12, slow: number = 26, signal: number = 9) {
  const closes = parsedData.value.closes
  const emaFast = calculateEMA(closes, fast)
  const emaSlow = calculateEMA(closes, slow)
  const dif: (number | null)[] = []
  const difValues: number[] = []
  for (let i = 0; i < closes.length; i++) {
    if (emaFast[i] != null && emaSlow[i] != null) {
      const d = Number((emaFast[i]! - emaSlow[i]!).toFixed(4))
      dif.push(d)
      difValues.push(d)
    } else {
      dif.push(null)
      difValues.push(0)
    }
  }
  const dea = calculateEMA(difValues, signal)
  const hist: (number | null)[] = []
  for (let i = 0; i < closes.length; i++) {
    if (dif[i] != null && dea[i] != null) {
      hist.push(Number(((dif[i]! - dea[i]!) * 2).toFixed(4)))
    } else {
      hist.push(null)
    }
  }
  return { dif, dea, hist }
}

function getLineSeries(name: string, data: (number | null)[], color: string, yAxisIndex: number = 0, width: number = 1) {
  return {
    name,
    type: 'line' as const,
    data,
    smooth: false,
    showSymbol: false,
    symbol: 'none',
    yAxisIndex,
    lineStyle: { width, opacity: 1 },
    itemStyle: { color },
    progressive: 0,
    progressiveThreshold: 0,
  }
}

function renderChart() {
  if (!chartRef.value) return
  if (chartRef.value.offsetWidth === 0 || chartRef.value.offsetHeight === 0) return
  if (!chart) {
    chart = echarts.init(chartRef.value)
  }
  const d = parsedData.value
  if (!d.times.length) {
    chart.clear()
    return
  }

  const showVol = props.showVolume
  const showMacd = props.showMACD

  const subPanels = (showVol ? 1 : 0) + (showMacd ? 1 : 0)

  const klineTop = 2
  const klineHeight = subPanels === 0 ? 88 : subPanels === 1 ? 62 : 48
  const subHeight = subPanels === 0 ? 0 : subPanels === 1 ? 14 : 12
  const gap = 4
  const sliderTop = 94

  const grids: any[] = [{ left: 60, right: 20, top: klineTop + '%', height: klineHeight + '%' }]
  const xAxes: any[] = [{ type: 'category', data: d.times, gridIndex: 0, boundaryGap: true, axisLine: { onZero: false }, axisLabel: { color: colors.label, fontSize: 11 } }]
  const yAxes: any[] = [{
    scale: true,
    gridIndex: 0,
    position: 'left',
    splitLine: { show: true, lineStyle: { color: '#e8e8e8', width: 0.5, type: 'dashed' } },
    axisLabel: { color: colors.label, fontSize: 11, formatter: (v: number) => v.toFixed(2) },
  }]
  const zoomXAxisIdx = [0]
  let gridIdx = 1

  if (showVol) {
    const volTop = klineTop + klineHeight + gap
    grids.push({ left: 60, right: 20, top: volTop + '%', height: subHeight + '%' })
    xAxes.push({ type: 'category', data: d.times, gridIndex: gridIdx, boundaryGap: true, axisLabel: { show: false } })
    yAxes.push({ scale: true, gridIndex: gridIdx, splitNumber: 2, axisLine: { onZero: false }, axisTick: { show: false }, splitLine: { show: false }, axisLabel: { color: colors.label, fontSize: 10, formatter: (v: number) => formatMoney(v, 0) } })
    zoomXAxisIdx.push(gridIdx)
    gridIdx++
  }

  if (showMacd) {
    const macdTop = klineTop + klineHeight + gap + (showVol ? subHeight + gap : 0)
    grids.push({ left: 60, right: 20, top: macdTop + '%', height: subHeight + '%' })
    xAxes.push({ type: 'category', data: d.times, gridIndex: gridIdx, boundaryGap: true, axisLabel: { show: false } })
    yAxes.push({ scale: true, gridIndex: gridIdx, splitNumber: 2, axisLine: { onZero: false }, axisTick: { show: false }, splitLine: { show: false }, axisLabel: { color: colors.label, fontSize: 10, formatter: (v: number) => v.toFixed(2) } })
    zoomXAxisIdx.push(gridIdx)
    gridIdx++
  }

  const dataLen = d.times.length
  const defaultVisible = 80
  const start = dataLen <= defaultVisible ? 0 : ((dataLen - defaultVisible) / dataLen) * 100
  const end = 100
  const minSpan = dataLen > 20 ? (20 / dataLen) * 100 : 100

  const series: any[] = [
    {
      name: '日K',
      type: 'candlestick',
      data: d.kline,
      xAxisIndex: 0,
      yAxisIndex: 0,
      barMaxWidth: 12,
      barMinWidth: 1,
      itemStyle: { color: colors.red, color0: colors.green, borderColor: colors.red, borderColor0: colors.green, borderWidth: 1 },
    },
  ]

  if (props.showMA) {
    series.push(
      getLineSeries('MA5', calculateMA(5), colors.ma5, 0, 1),
      getLineSeries('MA10', calculateMA(10), colors.ma10, 0, 1),
      getLineSeries('MA20', calculateMA(20), colors.ma20, 0, 1),
      getLineSeries('MA60', calculateMA(60), colors.ma60, 0, 1),
    )
  }

  if (props.showBoll) {
    const boll = calculateBoll(20)
    series.push(
      getLineSeries('BOLL上', boll.upper, colors.bollUp, 0, 1.5),
      getLineSeries('BOLL下', boll.lower, colors.bollDown, 0, 1.5),
      getLineSeries('BOLL中', boll.mid, colors.bollMid, 0, 1.5),
    )
  }

  let volGridIdx = -1
  if (showVol) {
    volGridIdx = 1
    series.push({
      name: '成交量',
      type: 'bar',
      xAxisIndex: volGridIdx,
      yAxisIndex: volGridIdx,
      data: d.vols,
      barMaxWidth: 12,
      barMinWidth: 1,
      itemStyle: {
        color: ((params: any) => {
          const kItem = d.kline[params.dataIndex]
          return kItem && kItem[1] >= kItem[0] ? colors.red : colors.green
        }) as any,
      },
    })
  }

  let macdGridIdx = -1
  if (showMacd) {
    macdGridIdx = showVol ? 2 : 1
    const macd = calculateMACD()
    series.push(
      getLineSeries('DIF', macd.dif, colors.dif, macdGridIdx, 1.2),
      getLineSeries('DEA', macd.dea, colors.dea, macdGridIdx, 1.2),
      {
        name: 'MACD',
        type: 'bar',
        xAxisIndex: macdGridIdx,
        yAxisIndex: macdGridIdx,
        data: macd.hist,
        barMaxWidth: 12,
        barMinWidth: 1,
        itemStyle: {
          color: ((params: any) => {
            const v = macd.hist[params.dataIndex]
            return v != null && v >= 0 ? colors.macdUp : colors.macdDown
          }) as any,
        },
      },
    )
  }

  const option = {
    animation: false,
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross', lineStyle: { color: colors.crosshair, width: 1 }, crossStyle: { color: colors.crosshair }, label: { color: colors.crosshair } },
      formatter(params: any) {
        const arr = Array.isArray(params) ? params : [params]
        const p = arr[0]
        if (!p || p.dataIndex == null) return ''
        const idx = p.dataIndex
        const kItem = d.kline[idx]
        if (!kItem) return ''
        const date = d.times[idx]
        const vol = d.vols[idx]
        const amt = d.amounts[idx]
        let html = `<div style="font-size:12px">日期: ${date}<hr style="margin:3px 0;border:none;border-top:1px solid #eee">`
        html += `开盘: ${kItem[0].toFixed(2)}<br/>`
        html += `收盘: ${kItem[1].toFixed(2)}<br/>`
        html += `最低: ${kItem[2].toFixed(2)}<br/>`
        html += `最高: ${kItem[3].toFixed(2)}<br/>`
        if (idx > 0) {
          const prevClose = d.kline[idx - 1]?.[1]
          if (prevClose) {
            const change = ((kItem[1] - prevClose) / prevClose * 100).toFixed(2)
            html += `涨跌幅: ${Number(change) >= 0 ? '+' : ''}${change}%<br/>`
          }
        }
        if (vol) html += `成交量: ${formatMoney(vol, 0)}<br/>`
        if (amt) html += `成交额: ${formatMoney(amt)}<br/>`
        if (showMacd) {
          const macd = calculateMACD()
          const difV = macd.dif[idx]
          const deaV = macd.dea[idx]
          const histV = macd.hist[idx]
          if (difV != null) html += `<hr style="margin:3px 0;border:none;border-top:1px solid #eee">DIF: ${difV.toFixed(3)}<br/>`
          if (deaV != null) html += `DEA: ${deaV.toFixed(3)}<br/>`
          if (histV != null) html += `MACD: ${histV.toFixed(3)}<br/>`
        }
        html += '</div>'
        return html
      },
    },
    axisPointer: { link: [{ xAxisIndex: 'all' }], label: { backgroundColor: 'rgba(255,255,255,0.8)', color: colors.crosshair } },
    grid: grids,
    xAxis: xAxes,
    yAxis: yAxes,
    dataZoom: [
      { type: 'inside', xAxisIndex: zoomXAxisIdx, start, end, minSpan, maxSpan: 100 },
      { type: 'slider', xAxisIndex: zoomXAxisIdx, top: sliderTop + '%', height: 12, start, end, minSpan, maxSpan: 100, borderColor: '#ddd', fillerColor: 'rgba(64,158,255,0.15)', handleStyle: { color: '#409eff' } },
    ],
    series,
  }

  chart.setOption(option, true)
}

let resizeHandler: (() => void) | null = null
let resizeObserver: ResizeObserver | null = null
let renderAttempts = 0
const maxRenderAttempts = 10

function tryRenderChart() {
  if (!chartRef.value) return
  const width = chartRef.value.offsetWidth
  const height = chartRef.value.offsetHeight
  if (width === 0 || height === 0) {
    if (renderAttempts < maxRenderAttempts) {
      renderAttempts++
      setTimeout(() => tryRenderChart(), 100)
    }
    return
  }
  renderChart()
}

onMounted(() => {
  renderAttempts = 0
  tryRenderChart()
  resizeHandler = () => chart?.resize()
  window.addEventListener('resize', resizeHandler)
  if (chartRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => {
      if (chart) {
        chart.resize()
      } else {
        tryRenderChart()
      }
    })
    resizeObserver.observe(chartRef.value)
  }
})

onBeforeUnmount(() => {
  if (resizeHandler) window.removeEventListener('resize', resizeHandler)
  if (resizeObserver) { resizeObserver.disconnect(); resizeObserver = null }
  chart?.dispose()
  chart = null
})

watch(() => [props.data, props.showMA, props.showBoll, props.showVolume, props.showMACD], () => renderChart(), { deep: true })
</script>
