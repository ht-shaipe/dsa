import { callApi } from './index'

export const agentApi = {
  /** 智能体对话 */
  chat: (message: string) => callApi('agent', 'chat', { message }),
  /** 获取可用模型列表 */
  models: () => callApi('agent', 'models'),
  /** 获取技能列表 */
  skills: () => callApi('agent', 'skills'),
  /** 获取策略列表 */
  strategies: () => callApi('agent', 'strategies'),
  /** 执行股票分析流水线 */
  pipeline: (code: string) => callApi('agent', 'pipeline', { code }),
}
