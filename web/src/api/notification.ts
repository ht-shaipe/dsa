import { callApi } from './index'

export const notificationApi = {
  /** 发送通知 */
  send: (params?: { channel?: string; title?: string; content?: string }) => callApi('notification', 'send', params || {}),
  /** 获取通知渠道列表 */
  channels: () => callApi('notification', 'channels'),
  /** 测试通知渠道 */
  test: (channel: string) => callApi('notification', 'test', { channel }),
}
