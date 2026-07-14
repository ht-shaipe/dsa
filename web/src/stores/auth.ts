import { defineStore } from 'pinia'
import { remoteLogin, remoteUpdateProfile, remoteGetProfile } from '@/api/index'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('dsa_token') || '',
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
        const result = await remoteLogin(mobile, password)
        if (result?.token) {
          this.token = result.token
          const user = result.user || {}
          this.userInfo = {
            mobile,
            name: user.name || user.nickname || '',
            avatar: user.avatar || user.avatar_url || '',
            ...user,
          }
          localStorage.setItem('dsa_token', this.token)
          localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
          return null
        }
        return '登录返回数据异常，未获得token'
      } catch (err: any) {
        return err?.message || '登录失败'
      }
    },
    async updateProfile(name: string, avatar?: string): Promise<boolean> {
      try {
        await remoteUpdateProfile(this.token, name, avatar)
        if (this.userInfo) {
          this.userInfo.name = name
          if (avatar) this.userInfo.avatar = avatar
          localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
        }
        return true
      } catch {
        return false
      }
    },
    async fetchProfile(): Promise<void> {
      if (!this.token) return
      try {
        const user = await remoteGetProfile(this.token)
        if (user) {
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
    logout() {
      this.token = ''
      this.userInfo = null
      localStorage.removeItem('dsa_token')
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
