import { ref } from 'vue'
import { systemApi } from '@/api/system'
import { stockPoolApi } from '@/api/stockPool'
import { useTaskStore } from '@/stores/task'

export type SyncPhase =
  | 'idle'
  | 'checking_quotes'
  | 'syncing_quotes'
  | 'quotes_up_to_date'
  | 'checking_daily'
  | 'syncing_daily'
  | 'daily_up_to_date'
  | 'check_complete'
  | 'error'

interface FreshnessResult {
  quote: { latestDate: string; isStale: boolean; daysGap: number }
  daily: { latestDate: string; isStale: boolean; daysGap: number }
  isTradingDay: boolean
  isTradingTime: boolean
  lastTradingDay: string
  needQuoteSync: boolean
  needDailySync: boolean
  syncRunning: boolean
}

export const syncPhase = ref<SyncPhase>('idle')
export const freshnessResult = ref<FreshnessResult | null>(null)

let checkTimer: ReturnType<typeof setInterval> | null = null
let lastCheckTime = 0
let visibilityHandler: (() => void) | null = null
let fadeTimer: ReturnType<typeof setTimeout> | null = null

function resetFade() {
  if (fadeTimer) { clearTimeout(fadeTimer); fadeTimer = null }
}

function scheduleFade(delay = 3000) {
  resetFade()
  fadeTimer = setTimeout(() => { syncPhase.value = 'idle' }, delay)
}

function waitForTaskCompletion(taskName: string, interval = 2000, timeout = 600000): Promise<void> {
  return new Promise((resolve) => {
    const start = Date.now()
    const poll = async () => {
      try {
        const taskStore = useTaskStore()
        const task = taskStore.tasks[taskName]
        if (!task || !task.running) {
          resolve()
          return
        }
      } catch { /* ignore */ }
      if (Date.now() - start > timeout) {
        resolve()
        return
      }
      setTimeout(poll, interval)
    }
    setTimeout(poll, interval)
  })
}

export async function checkAndSync(): Promise<void> {
  const taskStore = useTaskStore()
  if (taskStore.hasRunningTasks) return

  resetFade()

  try {
    syncPhase.value = 'checking_quotes'
    const res: FreshnessResult = await systemApi.checkFreshness() as any
    lastCheckTime = Date.now()
    freshnessResult.value = res

    if (res.syncRunning) {
      syncPhase.value = 'idle'
      return
    }

    if (res.needQuoteSync) {
      syncPhase.value = 'syncing_quotes'
      try {
        await stockPoolApi.refreshQuotes()
        taskStore.refreshAllStatus()
        await waitForTaskCompletion('refresh_quotes')
        taskStore.refreshAllStatus()
      } catch {
        syncPhase.value = 'error'
        scheduleFade(5000)
        return
      }
    } else {
      syncPhase.value = 'quotes_up_to_date'
    }

    if (res.needDailySync) {
      syncPhase.value = 'checking_daily'
      try {
        const res2: FreshnessResult = await systemApi.checkFreshness() as any
        freshnessResult.value = res2
        if (res2.needDailySync && !res2.syncRunning) {
          syncPhase.value = 'syncing_daily'
          await systemApi.syncDailyIncremental()
          taskStore.refreshAllStatus()
          await waitForTaskCompletion('sync_daily_incremental')
          taskStore.refreshAllStatus()
        }
      } catch {
        syncPhase.value = 'error'
        scheduleFade(5000)
        return
      }
    }

    const finalRes: FreshnessResult = await systemApi.checkFreshness().catch(() => res) as any
    freshnessResult.value = finalRes

    if (!finalRes.needQuoteSync && !finalRes.needDailySync) {
      syncPhase.value = 'check_complete'
    } else if (!finalRes.needDailySync) {
      syncPhase.value = 'daily_up_to_date'
    } else {
      syncPhase.value = 'check_complete'
    }

    scheduleFade(3000)
  } catch {
    syncPhase.value = 'error'
    scheduleFade(5000)
  }
}

export function startAutoCheck(intervalMinutes = 60): void {
  checkAndSync()

  checkTimer = setInterval(() => {
    checkAndSync()
  }, intervalMinutes * 60 * 1000)

  visibilityHandler = () => {
    if (!document.hidden) {
      const elapsed = Date.now() - lastCheckTime
      if (elapsed > 30 * 60 * 1000) {
        checkAndSync()
      }
    }
  }
  document.addEventListener('visibilitychange', visibilityHandler)
}

export function stopAutoCheck(): void {
  if (checkTimer) {
    clearInterval(checkTimer)
    checkTimer = null
  }
  if (visibilityHandler) {
    document.removeEventListener('visibilitychange', visibilityHandler)
    visibilityHandler = null
  }
  resetFade()
}

export function useAutoSync() {
  return { syncPhase, freshnessResult, checkAndSync, startAutoCheck, stopAutoCheck }
}
