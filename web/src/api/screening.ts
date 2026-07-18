import { callApi } from './index'

export const screeningApi = {
  status: () => callApi('screening', 'status'),
  strategies: () => callApi('screening', 'strategies'),
  hotspots: () => callApi('screening', 'hotspots'),
  hotspotDetail: (topic: string) => callApi('screening', 'hotspot_detail', { topic }),
  screen: (strategy?: string, macdParams?: Record<string, any>, limit?: number) =>
    callApi('screening', 'screen', { strategy, macd_params: macdParams, limit }),
  syncDaily: () => callApi('screening', 'sync_daily'),
  syncProgress: () => callApi('screening', 'sync_progress'),
  pauseSync: () => callApi('screening', 'pause_sync'),
  resumeSync: () => callApi('screening', 'resume_sync'),
  stopSync: () => callApi('screening', 'stop_sync'),
  history: (strategy?: string, limit?: number) =>
    callApi('screening', 'history', { strategy, limit }),
  historyDetail: (batchId: string) =>
    callApi('screening', 'history_detail', { batch_id: batchId }),
  compare: (batchId1: string, batchId2: string) =>
    callApi('screening', 'compare', { batch_id_1: batchId1, batch_id_2: batchId2 }),
  latestSummary: () => callApi('screening', 'latest_summary'),
}
