import { callApi } from './index'

export const intelligenceApi = {
  /** 获取情报源列表 */
  sources: () => callApi('intelligence', 'sources'),
  /** 创建情报源 */
  sourceCreate: (params: Record<string, any>) => callApi('intelligence', 'source_create', params),
  /** 更新情报源 */
  sourceUpdate: (params: Record<string, any>) => callApi('intelligence', 'source_update', params),
  /** 删除情报源 */
  sourceDelete: (id: number) => callApi('intelligence', 'source_delete', { id }),
  /** 测试情报源连接 */
  sourceTest: (url: string) => callApi('intelligence', 'source_test', { url }),
  /** 抓取情报源数据 */
  sourceFetch: (sourceId: number) => callApi('intelligence', 'source_fetch', { sourceId }),
  /** 抓取所有已启用情报源 */
  fetchEnabled: () => callApi('intelligence', 'fetch_enabled'),
  /** 获取情报条目列表 */
  items: (params?: { sourceId?: number; limit?: number }) => callApi('intelligence', 'items', params || {}),
  /** 获取情报模板列表 */
  templates: () => callApi('intelligence', 'templates'),
  /** 获取默认情报源配置 */
  defaults: () => callApi('intelligence', 'defaults'),
}
