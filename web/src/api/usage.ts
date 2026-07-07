import { callApi } from './index'

export const usageApi = {
  /** 获取用量汇总 */
  summary: (period?: string) => callApi('usage', 'summary', { period }),
  /** 获取用量仪表盘数据 */
  dashboard: () => callApi('usage', 'dashboard'),
  /** 获取用量记录列表 */
  records: (params?: { limit?: number; provider?: string }) => callApi('usage', 'records', params || {}),
}
