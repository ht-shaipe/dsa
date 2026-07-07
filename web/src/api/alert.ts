import { callApi } from './index'

export const alertApi = {
  /** 获取预警规则列表 */
  rules: (code?: string) => callApi('alert', 'rules', { code }),
  /** 创建预警规则 */
  ruleCreate: (params: { code: string; ruleType: string; name?: string; condition?: Record<string, any> }) => callApi('alert', 'rule_create', params),
  /** 更新预警规则 */
  ruleUpdate: (id: number, condition: Record<string, any>) => callApi('alert', 'rule_update', { id, condition }),
  /** 删除预警规则 */
  ruleDelete: (id: number) => callApi('alert', 'rule_delete', { id }),
  /** 启用预警规则 */
  ruleEnable: (id: number) => callApi('alert', 'rule_enable', { id }),
  /** 禁用预警规则 */
  ruleDisable: (id: number) => callApi('alert', 'rule_disable', { id }),
  /** 测试预警规则 */
  ruleTest: (code: string, condition: Record<string, any>) => callApi('alert', 'rule_test', { code, condition }),
  /** 获取预警触发记录 */
  triggers: (limit?: number) => callApi('alert', 'triggers', { limit }),
  /** 获取预警通知记录 */
  notifications: (limit?: number) => callApi('alert', 'notifications', { limit }),
}
