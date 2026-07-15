import { callApi } from './index'

export const stockApi = {
  search: (query: string) => callApi('stock', 'search', { query }),
  quote: (code: string) => callApi('stock', 'quote', { code }),
  quotes: (codes: string) => callApi('stock', 'quotes', { codes }),
  kline: (params: { code: string; period?: string; startDate?: string; endDate?: string; adjust?: string }) => callApi('stock', 'kline', params),
  history: (params: { code: string; days?: number }) => callApi('stock', 'history', params),
  info: (code: string) => callApi('stock', 'info', { code }),
  watchlist: () => callApi('stock', 'watchlist'),
  watchlistAdd: (code: string, name?: string) => callApi('stock', 'watchlist_add', { code, name }),
  watchlistRemove: (id: number, stock_code?: string) => callApi('stock', 'watchlist_remove', { id, stock_code }),
  watchlistUpdate: (params: { id: number; name?: string; group?: string; sort_order?: number; remark?: string }) => callApi('stock', 'watchlist_update', params),
  watchlistSync: (stocks: Array<{ code: string; name?: string }>) => callApi('stock', 'watchlist_sync', { stocks }),
  spot: () => callApi('stock', 'spot'),
  industries: () => callApi('stock', 'industries'),
  concepts: () => callApi('stock', 'concepts'),
}
