import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/LoginView.vue'),
      meta: { requiresAuth: false },
    },
    {
      path: '/',
      component: () => import('@/layout/AppLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        { path: '', name: 'Dashboard', component: () => import('@/views/DashboardView.vue') },
        { path: 'chat', name: 'Chat', component: () => import('@/views/ChatView.vue') },
        { path: 'screening', name: 'Screening', component: () => import('@/views/ScreeningView.vue') },
        { path: 'portfolio', name: 'Portfolio', component: () => import('@/views/PortfolioView.vue') },
        { path: 'decision-signals', name: 'DecisionSignals', component: () => import('@/views/DecisionSignalsView.vue') },
        { path: 'backtest', name: 'Backtest', component: () => import('@/views/BacktestView.vue') },
        { path: 'alerts', name: 'Alerts', component: () => import('@/views/AlertsView.vue') },
        { path: 'usage', name: 'Usage', component: () => import('@/views/UsageView.vue') },
        { path: 'settings', name: 'Settings', component: () => import('@/views/SettingsView.vue') },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  if (to.meta.requiresAuth !== false && !auth.isAuthenticated) {
    return { name: 'Login' }
  }
  if (to.name === 'Login' && auth.isAuthenticated) {
    return { name: 'Dashboard' }
  }
})

export default router
