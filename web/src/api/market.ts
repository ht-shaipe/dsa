import { callApi } from './index'

export const marketApi = {
  /** 获取市场概览 */
  overview: () => callApi('market', 'overview'),
  /** 获取市场点评 */
  review: () => callApi('market', 'review'),
  /** 获取热门板块 */
  hotSectors: () => callApi('market', 'hot_sectors'),
  /** 获取热门个股 */
  hotStocks: () => callApi('market', 'hot_stocks'),
  /** 获取指数数据 */
  index: (code?: string) => callApi('market', 'index', { code }),
  /** 获取交易日历 */
  calendar: (market?: string) => callApi('market', 'calendar', { market }),
}
