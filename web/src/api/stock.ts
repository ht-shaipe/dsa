import { callApi } from './index'

export const stockApi = {
  /** 股票搜索 */
  search: (query: string) => callApi('stock', 'search', { query }),
  /** 获取股票实时报价 */
  quote: (code: string) => callApi('stock', 'quote', { code }),
  /** 批量获取股票报价 */
  quotes: (codes: string) => callApi('stock', 'quotes', { codes }),
  /** 获取K线数据 */
  kline: (params: { code: string; period?: string; startDate?: string; endDate?: string; adjust?: string }) => callApi('stock', 'kline', params),
  /** 获取历史行情数据 */
  history: (params: { code: string; days?: number }) => callApi('stock', 'history', params),
  /** 获取股票基本信息 */
  info: (code: string) => callApi('stock', 'info', { code }),
  /** 获取自选股列表 */
  watchlist: (codes?: string) => callApi('stock', 'watchlist', { codes }),
  /** 获取全市场实时行情 */
  spot: () => callApi('stock', 'spot'),
  /** 获取行业列表 */
  industries: () => callApi('stock', 'industries'),
  /** 获取概念板块列表 */
  concepts: () => callApi('stock', 'concepts'),
}
