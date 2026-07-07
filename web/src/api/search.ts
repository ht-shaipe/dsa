import { callApi } from './index'

export const searchApi = {
  /** 搜索资讯 */
  search: (query: string, provider?: string) => callApi('search', 'search', { query, provider }),
  /** 获取股票相关新闻 */
  stockNews: (code: string) => callApi('search', 'stock_news', { code }),
  /** 获取搜索服务商列表 */
  providers: () => callApi('search', 'providers'),
}
