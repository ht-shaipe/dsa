/**
 * 股票相关公共工具函数
 */

/** 股价自适应格式化：大数简写，小数多精度 */
export function formatPrice(v: number | null | undefined): string {
  if (v == null || isNaN(v)) return '-'
  if (Math.abs(v) >= 10000) return (v / 10000).toFixed(2) + '万'
  if (Math.abs(v) >= 100) return v.toFixed(2)
  if (Math.abs(v) >= 1) return v.toFixed(3)
  return v.toFixed(3)
}

/** 成交量格式化（带"手"单位） */
export function formatVolume(v: number | null | undefined): string {
  if (v == null || isNaN(v)) return '-'
  if (v >= 1e8) return (v / 1e8).toFixed(2) + '亿手'
  if (v >= 1e4) return (v / 1e4).toFixed(2) + '万手'
  return Math.round(v) + '手'
}

/** 涨跌颜色类名（红涨绿跌） */
export function priceClass(pct: number | null | undefined): string {
  if (pct == null) return ''
  return pct > 0 ? 'price-up' : pct < 0 ? 'price-down' : ''
}

/** PE 估值高亮类名 */
export function peClass(pe: number | null | undefined): string {
  if (pe == null) return ''
  if (pe < 0 || pe > 100) return 'text-warning'
  return ''
}

/** 是否为 ST 股票 */
export function isST(name: string | null | undefined): boolean {
  if (!name) return false
  const u = name.toUpperCase()
  return u.includes('ST') || u.includes('*ST')
}
