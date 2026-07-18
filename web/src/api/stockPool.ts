import { callApi } from './index'

export const stockPoolApi = {
  list: (params?: { search?: string; page?: number; page_size?: number; status?: number }) =>
    callApi('stock_pool', 'list', params || {}),
  add: (params: { code: string; name?: string; marketId?: number; industry?: string }) =>
    callApi('stock_pool', 'add', params),
  remove: (params: { id?: number; stock_code?: string }) =>
    callApi('stock_pool', 'remove', params),
  batchRemove: (ids: number[]) =>
    callApi('stock_pool', 'batch_remove', { ids }),
  initPool: (params?: { boards?: string[]; exclude_st?: boolean; exclude_delisting?: boolean; exclude_new?: boolean }) =>
    callApi('stock_pool', 'init_pool', params || {}),
  count: () =>
    callApi('stock_pool', 'count'),
  refreshQuotes: () =>
    callApi('stock_pool', 'refresh_quotes'),
}
