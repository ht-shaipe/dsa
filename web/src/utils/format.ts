/**
 * 公共格式化工具函数
 */

/** 数字格式化，NaN 返回 '-' */
export function formatNum(v: any, digits: number = 2): string {
  const n = Number(v)
  return isNaN(n) ? '-' : n.toFixed(digits)
}

/** 金额格式化，自动转万/亿 */
export function formatMoney(v: any, digits: number = 2): string {
  const n = Number(v || 0)
  if (n >= 1e8) return (n / 1e8).toFixed(digits) + '亿'
  if (n >= 1e4) return (n / 1e4).toFixed(digits) + '万'
  return n.toFixed(digits)
}

/** 盈亏带正负号，默认 2 位小数 */
export function pnlText(val: number | undefined, digits: number = 2): string {
  const v = Number(val || 0)
  return (v >= 0 ? '+' : '') + v.toFixed(digits)
}

/** 盈亏百分比带正负号 */
export function pnlPercentText(val: number | undefined, digits: number = 2): string {
  const v = Number(val || 0)
  return (v >= 0 ? '+' : '') + v.toFixed(digits) + '%'
}

/** 盈亏 CSS 类名 */
export function pnlClass(val: number | undefined): string {
  const v = Number(val || 0)
  return v > 0 ? 'pnl-up' : v < 0 ? 'pnl-down' : ''
}

/** 盈亏方向 'up' | 'down' | '' */
export function pnlDir(val: number | undefined): 'up' | 'down' | '' {
  const v = Number(val || 0)
  return v > 0 ? 'up' : v < 0 ? 'down' : ''
}

/** 日期时间格式化 → YYYY-MM-DD HH:mm:ss */
export function formatDateTime(dt: any): string {
  if (!dt) return '-'
  const d = new Date(dt)
  if (isNaN(d.getTime())) return String(dt)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** 日期格式化 → YYYY-MM-DD */
export function formatDate(dt: any): string {
  if (!dt) return '-'
  return String(dt).slice(0, 10)
}

/** Token 数量格式化 */
export function formatTokens(v: number): string {
  if (v >= 1e6) return (v / 1e6).toFixed(1) + 'M'
  if (v >= 1e3) return (v / 1e3).toFixed(0) + 'K'
  return v.toLocaleString()
}
