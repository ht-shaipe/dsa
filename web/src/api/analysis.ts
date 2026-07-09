import { callApi, callApiWithTimeout } from './index'

export const analysisApi = {
  analyze: (code: string, name?: string) => callApiWithTimeout('analysis', 'analyze', { code, name }, 180000),
  batch: (codes: string) => callApiWithTimeout('analysis', 'batch', { codes }, 300000),
  report: (params: { id?: number; queryId?: string }) => callApi('analysis', 'report', params),
  list: (params?: { code?: string; limit?: number }) => callApi('analysis', 'list', params || {}),
  marketReview: (params?: Record<string, any>) => callApiWithTimeout('analysis', 'market-review', params || {}, 300000),
  historyList: (params?: { code?: string; limit?: number; offset?: number }) => callApi('analysis', 'history_list', params || {}),
  historyDetail: (id: number) => callApi('analysis', 'history_detail', { id }),
  historySearch: (keyword: string, limit?: number) => callApi('analysis', 'history_search', { keyword, limit: limit || 20 }),
  historyCompare: (id1: number, id2: number) => callApi('analysis', 'history_compare', { id1, id2 }),
}
