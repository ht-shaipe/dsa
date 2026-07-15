import { defineStore } from 'pinia'
import axios from 'axios'

function getApiBase(): string {
  if (typeof window !== 'undefined' && ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)) {
    return 'http://127.0.0.1:18080/api/v1'
  }
  return '/api/v1'
}

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('dsa_token') || '',
    remoteToken: localStorage.getItem('dsa_remote_token') || '',
    userInfo: null as any,
  }),
  getters: {
    isAuthenticated: (state) => !!state.token,
    displayName: (state) => state.userInfo?.name || state.userInfo?.mobile || '',
    avatarUrl: (state) => {
      const avatar = state.userInfo?.avatar
      if (!avatar) return undefined
      if (avatar.startsWith('http://') || avatar.startsWith('https://')) {
        return avatar
      }
      return `https://img.ecdata.cn${avatar.startsWith('/') ? '' : '/'}${avatar}`
    },
  },
  actions: {
    async loginWithRemote(mobile: string, password: string): Promise<string | null> {
      try {
        const { data } = await axios.post(`${getApiBase()}/auth/login`, { mobile, password })
        const body = data
        if (body?.code === 200 && body?.result) {
          const result = body.result
          this.token = result.local_token
          this.remoteToken = result.remote_token
          const user = result.user || {}
          this.userInfo = {
            mobile,
            name: user.name || user.nickname || '',
            avatar: user.avatar || user.avatar_url || '',
            ...user,
          }
          localStorage.setItem('dsa_token', this.token)
          localStorage.setItem('dsa_remote_token', this.remoteToken)
          localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
          return null
        }
        return body?.message || '登录返回数据异常，未获得token'
      } catch (err: any) {
        return err?.response?.data?.message || err?.message || '登录失败'
      }
    },
    async updateProfile(name: string, avatar?: string): Promise<boolean> {
      try {
        const { data } = await axios.post(`${getApiBase()}/auth/profile/update`, {
          token: this.remoteToken,
          name,
          avatar,
        })
        if (data?.code === 200) {
          if (this.userInfo) {
            this.userInfo.name = name
            if (avatar) this.userInfo.avatar = avatar
            localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
          }
          return true
        }
        return false
      } catch {
        return false
      }
    },
    async fetchProfile(): Promise<void> {
      if (!this.remoteToken) return
      try {
        const { data } = await axios.post(`${getApiBase()}/auth/profile`, {
          token: this.remoteToken,
        })
        if (data?.code === 200 && data?.result) {
          const user = data.result
          this.userInfo = {
            ...this.userInfo,
            name: user.name || user.nickname || this.userInfo?.name || '',
            mobile: user.mobile || this.userInfo?.mobile || '',
            avatar: user.avatar || user.avatar_url || this.userInfo?.avatar || '',
            ...user,
          }
          localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
        }
      } catch {
        // ignore — use cached data
      }
    },
    async changePassword(oldPassword: string, newPassword: string): Promise<string | null> {
      try {
        const { data } = await axios.post(`${getApiBase()}/auth/change-password`, {
          token: this.remoteToken,
          old_password: oldPassword,
          new_password: newPassword,
        })
        if (data?.code === 200) return null
        return data?.message || '修改密码失败'
      } catch (err: any) {
        return err?.response?.data?.message || err?.message || '修改密码失败'
      }
    },
    logout() {
      this.token = ''
      this.remoteToken = ''
      this.userInfo = null
      localStorage.removeItem('dsa_token')
      localStorage.removeItem('dsa_remote_token')
      localStorage.removeItem('dsa_user')
    },
    loadUserInfo() {
      const saved = localStorage.getItem('dsa_user')
      if (saved) {
        try { this.userInfo = JSON.parse(saved) } catch { /* ignore */ }
      }
    },
    init() {
      this.loadUserInfo()
    },
  },
})
