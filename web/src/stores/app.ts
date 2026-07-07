import { defineStore } from 'pinia'

export const useAppStore = defineStore('app', {
  state: () => ({
    isDark: localStorage.getItem('dsa_theme') === 'dark',
    sidebarCollapsed: false,
  }),
  actions: {
    toggleTheme() {
      this.isDark = !this.isDark
      localStorage.setItem('dsa_theme', this.isDark ? 'dark' : 'light')
      document.documentElement.classList.toggle('dark', this.isDark)
    },
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
    },
    initTheme() {
      document.documentElement.classList.toggle('dark', this.isDark)
    },
  },
})
