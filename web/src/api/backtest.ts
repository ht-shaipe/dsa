import { callApi } from './index'

export const backtestApi = {
  /** 执行回测 */
  run: (analysisId: number) => callApi('backtest', 'run', { analysisId }),
  /** 获取回测记录列表 */
  list: (params?: { code?: string; limit?: number }) => callApi('backtest', 'list', params || {}),
  /** 获取回测详情 */
  detail: (id: number) => callApi('backtest', 'detail', { id }),
  /** 获取信号验证结果 */
  outcomes: (params?: { signalId?: number; limit?: number }) => callApi('backtest', 'outcomes', params || {}),
  /** 获取回测绩效统计 */
  performance: (code?: string) => callApi('backtest', 'performance', { code }),
}
