import { defineStore } from 'pinia'
import { authApi } from '@/api/auth'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('dsa_token') || '',
    authEnabled: false,
  }),
  getters: {
    isAuthenticated: (state) => !!state.token,
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
      localStorage.removeItem('dsa_token')
      if (this.authEnabled) {
        authApi.logout().catch(() => {})
      }
    },
  },
})
