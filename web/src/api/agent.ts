import { callApi } from './index'

export const agentApi = {
  chat: (message: string) => callApi('agent', 'chat', { message }),
  models: () => callApi('agent', 'models'),
  skills: () => callApi('agent', 'skills'),
  strategies: () => callApi('agent', 'strategies'),
  pipeline: (code: string) => callApi('agent', 'pipeline', { code }),
  history: (sessionId?: string) => callApi('agent', 'history', { session_id: sessionId || '' }),
}
