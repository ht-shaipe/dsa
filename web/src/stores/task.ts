import { defineStore } from 'pinia'
import { connectSSE, getTaskSSEUrl } from '@/utils/sse'
import { systemApi } from '@/api/system'
import { screeningApi } from '@/api/screening'

export interface TaskProgress {
  task: string
  running: boolean
  paused: boolean
  total: number
  done: number
  failed: number
  phase: string
  current_code?: string
  current_name?: string
}

const TASK_LABELS: Record<string, string> = {
  init_daily_data: '日线数据初始化',
  sync_daily: '日线数据同步',
  init_stock_pool: '股票池初始化',
}

export function getTaskLabel(task: string): string {
  return TASK_LABELS[task] || task
}

export function getPhaseLabel(phase: string): string {
  if (phase === 'preparing') return '正在获取市场数据...'
  if (phase === 'fetching') return '正在拉取日线数据...'
  if (phase === 'writing') return '正在写入股票池...'
  if (phase === 'calculating_indicators') return '正在计算技术指标...'
  if (phase.startsWith('calculating_indicators')) return '正在计算技术指标...'
  if (phase === 'done') return '已完成'
  return phase
}

export const useTaskStore = defineStore('task', {
  state: () => ({
    tasks: {} as Record<string, TaskProgress>,
    sseConnected: false,
    _sseHandle: null as { close: () => void } | null,
    _reconnectTimer: null as ReturnType<typeof setTimeout> | null,
    _reconnectAttempts: 0,
    _stallCheckTimer: null as ReturnType<typeof setInterval> | null,
    _lastDone: {} as Record<string, number>,
  }),

  getters: {
    hasRunningTasks(): boolean {
      return Object.values(this.tasks).some((t) => t.running)
    },
    runningTasks(): TaskProgress[] {
      return Object.values(this.tasks).filter((t) => t.running)
    },
    stalledTasks(): TaskProgress[] {
      return Object.values(this.tasks).filter((t) => {
        if (!t.running || t.paused) return false
        const lastDone = this._lastDone[t.task]
        if (lastDone === undefined) return false
        return t.done === lastDone && t.done < t.total
      })
    },
  },

  actions: {
    connect() {
      this.disconnect()
      this._reconnectAttempts = 0
      this._doConnect()
      this._startStallCheck()
    },

    _startStallCheck() {
      this._stopStallCheck()
      this._stallCheckTimer = setInterval(() => {
        for (const task of Object.values(this.tasks)) {
          if (task.running && !task.paused) {
            const prev = this._lastDone[task.task]
            if (prev !== undefined && task.done === prev && task.done < task.total) {
              // stalled - trigger a manual refresh to confirm
              this.refreshAllStatus()
            }
            this._lastDone[task.task] = task.done
          }
        }
      }, 30000)
    },

    _stopStallCheck() {
      if (this._stallCheckTimer) {
        clearInterval(this._stallCheckTimer)
        this._stallCheckTimer = null
      }
    },

    _doConnect() {
      const url = getTaskSSEUrl()
      this._sseHandle = connectSSE({
        url,
        onEvent: (event) => {
          if (event.task) {
            this.tasks[event.task] = {
              task: event.task,
              running: event.running || false,
              paused: event.paused || false,
              total: event.total || 0,
              done: event.done || 0,
              failed: event.failed || 0,
              phase: event.phase || '',
              current_code: event.current_code || '',
              current_name: event.current_name || '',
            }
          }
          this.sseConnected = true
          this._reconnectAttempts = 0
        },
        onError: () => {
          this.sseConnected = false
          this._scheduleReconnect()
        },
        onDone: () => {
          this.sseConnected = false
          this._scheduleReconnect()
        },
      })
    },

    _scheduleReconnect() {
      if (this._reconnectTimer) return
      const delay = Math.min(1000 * Math.pow(2, this._reconnectAttempts), 30000)
      this._reconnectAttempts++
      this._reconnectTimer = setTimeout(() => {
        this._reconnectTimer = null
        this._doConnect()
      }, delay)
    },

    disconnect() {
      if (this._sseHandle) {
        this._sseHandle.close()
        this._sseHandle = null
      }
      if (this._reconnectTimer) {
        clearTimeout(this._reconnectTimer)
        this._reconnectTimer = null
      }
      this._stopStallCheck()
      this.sseConnected = false
    },

    async pauseTask(task: string) {
      try {
        if (task === 'sync_daily') {
          await screeningApi.pauseSync()
        } else {
          await systemApi.pauseSync()
        }
      } catch { /* ignore */ }
    },

    async resumeTask(task: string) {
      try {
        if (task === 'sync_daily') {
          await screeningApi.resumeSync()
        } else {
          await systemApi.resumeSync()
        }
      } catch { /* ignore */ }
    },

    async stopTask(task: string) {
      try {
        if (task === 'sync_daily') {
          await screeningApi.stopSync()
        } else {
          await systemApi.stopSync()
        }
      } catch { /* ignore */ }
    },

    async refreshAllStatus() {
      try {
        const sysRes: any = await systemApi.syncStatus().catch(() => null)
        if (sysRes?.running) {
          this.tasks[sysRes.task || 'init_daily_data'] = {
            task: sysRes.task || 'init_daily_data',
            running: sysRes.running,
            paused: sysRes.paused || false,
            total: sysRes.total || 0,
            done: sysRes.done || 0,
            failed: sysRes.failed || 0,
            phase: sysRes.phase || '',
            current_code: sysRes.current_code || '',
            current_name: sysRes.current_name || '',
          }
        } else {
          if (this.tasks['init_daily_data']) delete this.tasks['init_daily_data']
          if (this.tasks['init_stock_pool']) delete this.tasks['init_stock_pool']
        }

        const scrRes: any = await screeningApi.syncProgress().catch(() => null)
        if (scrRes?.running) {
          this.tasks['sync_daily'] = {
            task: 'sync_daily',
            running: scrRes.running,
            paused: scrRes.paused || false,
            total: scrRes.total || 0,
            done: scrRes.done || 0,
            failed: scrRes.failed || 0,
            phase: scrRes.phase || '',
          }
        } else if (this.tasks['sync_daily']) {
          delete this.tasks['sync_daily']
        }
      } catch { /* ignore */ }
    },
  },
})
