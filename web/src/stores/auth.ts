import { defineStore } from 'pinia'
import { authApi } from '@/api/auth'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    token: localStorage.getItem('dsa_token') || '',
    authEnabled: true,
  }),
  getters: {
    isAuthenticated: (state) => !!state.token,
  },
  actions: {
    async checkStatus() {
      try {
        const res: any = await authApi.status()
        this.authEnabled = res.authEnabled ?? true
        if (!res.authEnabled) {
          this.token = ''
          localStorage.removeItem('dsa_token')
        }
      } catch {
        this.authEnabled = false
      }
    },
    async login(password: string): Promise<boolean> {
      try {
        const res: any = await authApi.login(password)
        if (res.authenticated) {
          this.token = res.token
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
