import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import router from '@/router'
import { ElMessage } from 'element-plus'

function getApiBase(): string {
  if (typeof window !== 'undefined' && ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)) {
    return 'http://127.0.0.1:18080/api/v1'
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
  if (auth.token && auth.token !== 'no-auth-required') {
    config.headers.Authorization = `Bearer ${auth.token}`
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

const remoteAuth = axios.create({
  baseURL: 'https://auth.htui.cc/api/pas',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' },
})

export interface RemoteLoginResult {
  token: string
  user?: any
}

export function remoteLogin(mobile: string, password: string): Promise<RemoteLoginResult> {
  return remoteAuth.post('/user/login', { mobile, password }).then((res) => {
    const body = res.data
    if (body.code === 200 || body.code === 0) {
      return body.result || body.data || body
    }
    return Promise.reject(new Error(body.message || '登录失败'))
  })
}

export function remoteRegister(mobile: string, password: string, name?: string): Promise<any> {
  const params: Record<string, any> = { mobile, password }
  if (name) params.name = name
  return remoteAuth.post('/user/register', params).then((res) => {
    const body = res.data
    if (body.code === 200 || body.code === 0) {
      return body.result || body.data || body
    }
    return Promise.reject(new Error(body.message || '注册失败'))
  })
}

export default api
