import { callApi } from './index'

export const backtestApi = {
  evaluate: (signal_id: number) => callApi('backtest', 'evaluate', { signalId }),
  evaluateBatch: (limit?: number) => callApi('backtest', 'evaluate_batch', { limit: limit || 50 }),
  summary: (params?: { code?: string; horizon?: string }) => callApi('backtest', 'summary', params || {}),
  detail: (id: number) => callApi('backtest', 'detail', { id }),
  list: (params?: { code?: string; outcome?: string; limit?: number; offset?: number }) => callApi('backtest', 'list', params || {}),
}
