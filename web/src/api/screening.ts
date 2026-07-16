import { callApi } from './index'

export const screeningApi = {
  status: () => callApi('screening', 'status'),
  strategies: () => callApi('screening', 'strategies'),
  hotspots: () => callApi('screening', 'hotspots'),
  hotspotDetail: (topic: string) => callApi('screening', 'hotspot_detail', { topic }),
  screen: (strategy?: string) => callApi('screening', 'screen', { strategy }),
  syncDaily: () => callApi('screening', 'sync_daily'),
  syncProgress: () => callApi('screening', 'sync_progress'),
  pauseSync: () => callApi('screening', 'pause_sync'),
  resumeSync: () => callApi('screening', 'resume_sync'),
  stopSync: () => callApi('screening', 'stop_sync'),
}
