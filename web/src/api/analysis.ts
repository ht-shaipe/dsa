import { callApi } from './index'

export const analysisApi = {
  /** 分析单只股票 */
  analyze: (code: string, name?: string) => callApi('analysis', 'analyze', { code, name }),
  /** 批量分析股票 */
  batch: (codes: string) => callApi('analysis', 'batch', { codes }),
  /** 获取分析报告 */
  report: (params: { id?: number; queryId?: string }) => callApi('analysis', 'report', params),
  /** 获取分析记录列表 */
  list: (params?: { code?: string; limit?: number }) => callApi('analysis', 'list', params || {}),
  /** 市场综述分析 */
  marketReview: (params?: Record<string, any>) => callApi('analysis', 'market-review', params || {}),
}
