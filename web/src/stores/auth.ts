import { defineStore } from 'pinia'
import { authApi } from '@/api/auth'
import { remoteLogin } from '@/api/index'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('dsa_token') || '',
    authEnabled: false,
    userInfo: null as any,
  }),
  getters: {
    isAuthenticated: (state) => !!state.token,
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
    async checkStatus() {
      try {
        const data: any = await authApi.status()
        this.authEnabled = data?.authEnabled ?? false
        if (!this.authEnabled && !this.token) {
          this.token = 'no-auth-required'
          localStorage.setItem('dsa_token', this.token)
        }
      } catch {
        this.authEnabled = false
      }
    },
    async loginWithRemote(mobile: string, password: string): Promise<boolean> {
      try {
        const result = await remoteLogin(mobile, password)
        if (result?.token) {
          this.token = result.token
          this.userInfo = result.user || { mobile }
          localStorage.setItem('dsa_token', this.token)
          localStorage.setItem('dsa_user', JSON.stringify(this.userInfo))
          return true
        }
        return false
      } catch {
        return false
      }
    },
    async login(password: string): Promise<boolean> {
      try {
        const data: any = await authApi.login(password)
        if (data?.authenticated) {
          this.token = data.token || 'no-auth-required'
          localStorage.setItem('dsa_token', this.token)
          return true
        }
        return false
      } catch {
        return false
      }
    },
    logout() {
      this.token = ''
      this.userInfo = null
      localStorage.removeItem('dsa_token')
      localStorage.removeItem('dsa_user')
      if (this.authEnabled) {
        authApi.logout().catch(() => {})
      }
    },
    loadUserInfo() {
      const saved = localStorage.getItem('dsa_user')
      if (saved) {
        try { this.userInfo = JSON.parse(saved) } catch { /* ignore */ }
      }
    },
  },
})
