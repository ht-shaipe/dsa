import { callApi } from './index'

export const alertApi = {
  rules: (code?: string) => callApi('alert', 'rules', { code }),
  ruleCreate: (params: { code: string; ruleType: string; name?: string; condition?: Record<string, any> }) => callApi('alert', 'rule_create', {
    code: params.code,
    ruleType: params.ruleType,
    name: params.name,
    condition: JSON.stringify(params.condition || {}),
  }),
  ruleUpdate: (id: number, condition: Record<string, any>) => callApi('alert', 'rule_update', { id, condition: JSON.stringify(condition) }),
  ruleDelete: (id: number) => callApi('alert', 'rule_delete', { id }),
  ruleEnable: (id: number) => callApi('alert', 'rule_enable', { id }),
  ruleDisable: (id: number) => callApi('alert', 'rule_disable', { id }),
  ruleTest: (code: string, condition: Record<string, any>) => callApi('alert', 'rule_test', { code, condition: JSON.stringify(condition) }),
  triggers: (limit?: number) => callApi('alert', 'triggers', { limit }),
  notifications: (limit?: number) => callApi('alert', 'notifications', { limit }),
}
