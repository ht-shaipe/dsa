import { callApi } from './index'

export const authApi = {
  /** 获取认证状态 */
  status: () => callApi('auth', 'status'),
  /** 登录 */
  login: (password: string) => callApi('auth', 'login', { password }),
  /** 退出登录 */
  logout: () => callApi('auth', 'logout'),
  /** 修改密码 */
  changePassword: (oldPassword: string, newPassword: string) => callApi('auth', 'change_password', { oldPassword, newPassword }),
  /** 获取用户设置 */
  settings: () => callApi('auth', 'settings'),
}
