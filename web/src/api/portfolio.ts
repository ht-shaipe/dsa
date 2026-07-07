import { callApi } from './index'

export const portfolioApi = {
  /** 获取账户列表 */
  accounts: () => callApi('portfolio', 'accounts'),
  /** 添加持仓 */
  add: (params: { account_id: number; code: string; price: number; quantity: number; name?: string; commission?: number; remark?: string }) => callApi('portfolio', 'add', params),
  /** 减仓/清仓 */
  remove: (params: { account_id: number; code: string; price?: number; quantity?: number; commission?: number; remark?: string }) => callApi('portfolio', 'remove', params),
  /** 获取组合汇总 */
  summary: (accountId?: number) => callApi('portfolio', 'summary', { accountId }),
  /** 获取持仓列表 */
  positions: (accountId?: number) => callApi('portfolio', 'positions', { accountId }),
  /** 获取交易记录 */
  trades: (params?: { accountId?: number; limit?: number }) => callApi('portfolio', 'trades', params || {}),
  /** 获取账户快照 */
  snapshot: (account_id: number) => callApi('portfolio', 'snapshot', { accountId }),
}
