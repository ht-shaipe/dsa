import { callApi, callApiWithTimeout } from './index'

export const systemApi = {
  get: () => callApi('system', 'get'),
  reload: () => callApi('system', 'reload'),
  save: (config: any) => callApi('system', 'save', { config }),
  validate: (config: any) => callApi('system', 'validate', { config }),
  exportConfig: () => callApi('system', 'export_config'),
  importConfig: (config: string) => callApi('system', 'import_config', { config }),
  testLlm: () => callApi('system', 'test_llm'),
  discoverModels: () => callApi('system', 'discover_models'),
  initDailyData: () => callApi('system', 'init_daily_data'),
  syncStatus: () => callApi('system', 'sync_status'),
  cleanDailyData: () => callApi('system', 'clean_daily_data'),
  dailyDataStats: () => callApi('system', 'daily_data_stats'),
  exportDailyData: () => callApi('system', 'export_daily_data'),
  importDailyData: (data: any) => callApiWithTimeout('system', 'import_daily_data', { data }, 300000),
}
