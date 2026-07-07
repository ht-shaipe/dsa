import { callApi } from './index'

export const schedulerApi = {
  /** 启动调度器 */
  start: () => callApi('scheduler', 'start'),
  /** 停止调度器 */
  stop: () => callApi('scheduler', 'stop'),
  /** 获取调度器状态 */
  status: () => callApi('scheduler', 'status'),
  /** 获取定时任务列表 */
  jobs: () => callApi('scheduler', 'jobs'),
  /** 手动触发任务 */
  trigger: (params?: { type?: string; codes?: string }) => callApi('scheduler', 'trigger', params || {}),
}
