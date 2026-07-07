import { callApi } from './index'

export const systemApi = {
  /** 获取系统配置 */
  get: () => callApi('system', 'get'),
  /** 重载系统配置 */
  reload: () => callApi('system', 'reload'),
  /** 校验系统配置 */
  validate: (config: any) => callApi('system', 'validate', { config }),
  /** 导出系统配置 */
  exportConfig: () => callApi('system', 'export_config'),
  /** 导入系统配置 */
  importConfig: (config: string) => callApi('system', 'import_config', { config }),
  /** 测试LLM连接 */
  testLlm: () => callApi('system', 'test_llm'),
  /** 发现可用模型 */
  discoverModels: () => callApi('system', 'discover_models'),
}
