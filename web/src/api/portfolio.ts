import { callApi } from './index'

export const portfolioApi = {
  accounts: () => callApi('portfolio', 'accounts'),
  add: (params: { accountId: number; code: string; price: number; quantity: number; name?: string; commission?: number; remark?: string; tradeDate?: string }) => callApi('portfolio', 'add', params),
  remove: (params: { accountId: number; code: string; price?: number; quantity?: number; commission?: number; remark?: string; tradeDate?: string }) => callApi('portfolio', 'remove', params),
  summary: (accountId?: number) => callApi('portfolio', 'summary', { accountId }),
  positions: (accountId?: number) => callApi('portfolio', 'positions', { accountId }),
  trades: (params?: { accountId?: number; limit?: number }) => callApi('portfolio', 'trades', params || {}),
  snapshot: (accountId: number) => callApi('portfolio', 'snapshot', { accountId }),
}
