import { defineStore } from 'pinia'

export type ThemeMode = 'system' | 'light' | 'dark'

export const useAppStore = defineStore('app', {
  state: () => ({
    themeMode: (localStorage.getItem('dsa_theme_mode') as ThemeMode) || 'system',
    sidebarCollapsed: false,
  }),
  getters: {
    isDark(): boolean {
      if (this.themeMode === 'system') {
        return window.matchMedia('(prefers-color-scheme: dark)').matches
      }
      return this.themeMode === 'dark'
    },
  },
  actions: {
    setTheme(mode: ThemeMode) {
      this.themeMode = mode
      localStorage.setItem('dsa_theme_mode', mode)
      this.applyTheme()
    },
    toggleTheme() {
      const modes: ThemeMode[] = ['system', 'light', 'dark']
      const currentIndex = modes.indexOf(this.themeMode)
      this.setTheme(modes[(currentIndex + 1) % modes.length])
    },
    applyTheme() {
      const isDark = this.isDark
      document.documentElement.classList.toggle('dark', isDark)
      // 设置 meta theme-color
      const metaThemeColor = document.querySelector('meta[name="theme-color"]')
      if (metaThemeColor) {
        metaThemeColor.setAttribute('content', isDark ? '#1a1a1a' : '#ffffff')
      }
    },
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
    },
    initTheme() {
      this.applyTheme()
      // 监听系统主题变化
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (this.themeMode === 'system') {
          this.applyTheme()
        }
      })
    },
  },
})
