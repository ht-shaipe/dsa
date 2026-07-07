import { callApi } from './index'

export const decisionApi = {
  /** 创建决策信号 */
  create: (params: Record<string, any>) => callApi('decision', 'create', params),
  /** 获取决策信号列表 */
  list: (params?: { code?: string; status?: string; action?: string; holdingOnly?: boolean; limit?: number }) => callApi('decision', 'list', params || {}),
  /** 获取最新决策信号 */
  latest: (code: string) => callApi('decision', 'latest', { code }),
  /** 获取决策信号详情 */
  detail: (id: number) => callApi('decision', 'detail', { id }),
  /** 更新决策信号状态 */
  updateStatus: (id: number, status: string) => callApi('decision', 'update_status', { id, status }),
  /** 获取信号验证结果 */
  outcomes: (params?: { signalId?: number; limit?: number }) => callApi('decision', 'outcomes', params || {}),
  /** 提交决策反馈 */
  feedback: (signalId: number, feedback: string, rating?: number) => callApi('decision', 'feedback', { signalId, feedback, rating }),
  /** 评估决策结果 */
  evaluateOutcomes: (evalWindow?: number) => callApi('decision', 'evaluate_outcomes', { evalWindow }),
  /** 获取决策统计 */
  stats: (code?: string) => callApi('decision', 'stats', { code }),
}
