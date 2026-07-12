import { callApi, callApiWithTimeout } from './index'

export const decisionApi = {
  create: (params: Record<string, any>) => callApi('decision', 'create', params),
  list: (params?: { code?: string; status?: string; action?: string; holdingOnly?: boolean; limit?: number }) => callApi('decision', 'list', params || {}),
  latest: (code: string) => callApi('decision', 'latest', { code }),
  detail: (id: number) => callApi('decision', 'detail', { id }),
  updateStatus: (id: number, status: string) => callApi('decision', 'update_status', { id, status }),
  outcomes: (params?: { signalId?: number; limit?: number }) => callApi('decision', 'outcomes', params || {}),
  feedback: (signalId: number, feedback: string, rating?: number) => callApi('decision', 'feedback', { signalId, feedback, rating }),
  evaluateOutcomes: (evalWindow?: number) => callApi('decision', 'evaluate_outcomes', { evalWindow }),
  stats: (code?: string) => callApi('decision', 'stats', { code }),
  extractBatch: (limit?: number) => callApiWithTimeout('decision_extractor', 'extract_batch', { limit: limit || 10 }, 120000),
  reassess: () => callApi('decision', 'reassess', {}),
}
