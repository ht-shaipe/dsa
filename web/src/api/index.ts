import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import router from '@/router'
import { ElMessage } from 'element-plus'

export function getApiBase(): string {
  if (typeof window !== 'undefined' && ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)) {
    const port = (window as any).__DSA_API_PORT__ || 18080
    return `http://127.0.0.1:${port}/api/v1`
  }
  return '/api/v1'
}

const api = axios.create({
  baseURL: getApiBase(),
  timeout: 60000,
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.request.use((config) => {
  const auth = useAuthStore()
  if (auth.token) {
    config.headers.Authorization = auth.token
  }
  return config
})

api.interceptors.response.use(
  (res) => {
    const body = res.data
    if (!body || typeof body !== 'object' || !('code' in body)) {
      return body
    }
    if (body.code !== 200) {
      const msg = body.message || '请求失败'
      ElMessage.error(msg)
      return Promise.reject(new Error(msg))
    }
    return body.result
  },
  (err) => {
    if (err.response?.status === 401) {
      const auth = useAuthStore()
      auth.logout()
      router.push('/login')
      ElMessage.error('登录已过期，请重新登录')
    } else {
      const msg = err.response?.data?.message || err.response?.data?.error || err.message || '请求失败'
      ElMessage.error(msg)
    }
    return Promise.reject(err)
  },
)

export function callApi(module: string, method: string, params: Record<string, any> = {}): Promise<any> {
  return api.post(`/${module}/${method}`, params)
}

export function callApiWithTimeout(module: string, method: string, params: Record<string, any> = {}, timeout: number = 120000): Promise<any> {
  return api.post(`/${module}/${method}`, params, { timeout })
}

export default api
