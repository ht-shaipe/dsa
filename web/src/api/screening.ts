import { callApi } from './index'

export const screeningApi = {
  /** 获取筛选任务状态 */
  status: () => callApi('screening', 'status'),
  /** 获取筛选策略列表 */
  strategies: () => callApi('screening', 'strategies'),
  /** 获取热点话题列表 */
  hotspots: () => callApi('screening', 'hotspots'),
  /** 获取热点话题详情 */
  hotspotDetail: (topic: string) => callApi('screening', 'hotspot_detail', { topic }),
  /** 执行选股筛选 */
  screen: (strategy?: string) => callApi('screening', 'screen', { strategy }),
}
