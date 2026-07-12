export interface TradingTimer {
  start: () => void
  stop: () => void
  isActive: () => boolean
}

const HOLIDAYS_2025: string[] = [
  '2025-01-01', '2025-01-28', '2025-01-29', '2025-01-30', '2025-01-31',
  '2025-02-03', '2025-02-04', '2025-04-04', '2025-05-01', '2025-05-02',
  '2025-05-05', '2025-06-02', '2025-10-01', '2025-10-02', '2025-10-03',
  '2025-10-06', '2025-10-07', '2025-10-08',
]

const HOLIDAYS_2026: string[] = [
  '2026-01-01', '2026-02-16', '2026-02-17', '2026-02-18', '2026-02-19',
  '2026-02-20', '2026-04-06', '2026-05-01', '2026-05-04', '2026-05-05',
  '2026-06-19', '2026-10-01', '2026-10-02', '2026-10-05', '2026-10-06',
  '2026-10-07', '2026-10-08',
]

const HOLIDAYS = new Set([...HOLIDAYS_2025, ...HOLIDAYS_2026])

const EXTRA_TRADE_DAYS: string[] = [
  '2025-01-26', '2025-02-08', '2025-04-27', '2025-09-28', '2025-10-11',
  '2026-01-24', '2026-02-14', '2026-09-27', '2026-10-10',
]

const EXTRA_SET = new Set(EXTRA_TRADE_DAYS)

function isTradingDay(date: Date): boolean {
  const ymd = formatDate(date)
  if (EXTRA_SET.has(ymd)) return true
  if (HOLIDAYS.has(ymd)) return false
  const dow = date.getDay()
  return dow !== 0 && dow !== 6
}

function isTradingTime(date: Date): boolean {
  if (!isTradingDay(date)) return false
  const h = date.getHours()
  const m = date.getMinutes()
  const t = h * 60 + m
  return (t >= 9 * 60 + 15 && t <= 11 * 60 + 30) || (t >= 13 * 60 && t <= 15 * 60)
}

function formatDate(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${dd}`
}

function msUntilNextTradingSession(now: Date): number {
  const today = new Date(now)
  today.setHours(0, 0, 0, 0)

  for (let i = 0; i < 7; i++) {
    const day = new Date(today.getTime() + i * 86400000)
    if (!isTradingDay(day)) continue

    const morningStart = new Date(day)
    morningStart.setHours(9, 15, 0, 0)
    const morningEnd = new Date(day)
    morningEnd.setHours(11, 30, 0, 0)
    const afternoonStart = new Date(day)
    afternoonStart.setHours(13, 0, 0, 0)
    const afternoonEnd = new Date(day)
    afternoonEnd.setHours(15, 0, 0, 0)

    if (now < morningStart) return morningStart.getTime() - now.getTime()
    if (now >= morningStart && now < morningEnd) return 0
    if (now >= morningEnd && now < afternoonStart) return afternoonStart.getTime() - now.getTime()
    if (now >= afternoonStart && now < afternoonEnd) return 0
  }

  return 86400000
}

export function useTradingInterval(
  callback: () => void,
  intervalMs: number = 10000,
): TradingTimer {
  let timerId: ReturnType<typeof setTimeout> | null = null
  let running = false
  let pausedByHidden = false

  function clearTimer() {
    if (timerId !== null) {
      clearTimeout(timerId)
      timerId = null
    }
  }

  function scheduleNext() {
    if (!running) return

    if (document.hidden) {
      pausedByHidden = true
      return
    }

    pausedByHidden = false

    const now = new Date()
    if (isTradingTime(now)) {
      timerId = setTimeout(() => {
        if (!running || document.hidden) {
          if (document.hidden) pausedByHidden = true
          return
        }
        callback()
        scheduleNext()
      }, intervalMs)
    } else {
      const wait = msUntilNextTradingSession(now)
      timerId = setTimeout(() => {
        if (!running || document.hidden) {
          if (document.hidden) pausedByHidden = true
          return
        }
        callback()
        scheduleNext()
      }, Math.max(wait, 1000))
    }
  }

  function onVisibilityChange() {
    if (!running) return
    if (!document.hidden && pausedByHidden) {
      pausedByHidden = false
      callback()
      scheduleNext()
    }
  }

  return {
    start() {
      if (running) return
      running = true
      document.addEventListener('visibilitychange', onVisibilityChange)
      if (document.hidden) {
        pausedByHidden = true
      } else {
        callback()
        scheduleNext()
      }
    },
    stop() {
      running = false
      pausedByHidden = false
      clearTimer()
      document.removeEventListener('visibilitychange', onVisibilityChange)
    },
    isActive() {
      return running
    },
  }
}

export { isTradingDay, isTradingTime, msUntilNextTradingSession }
