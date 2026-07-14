import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import router from '@/router'
import { ElMessage } from 'element-plus'

export function getApiBase(): string {
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
  if (auth.token && auth.token !== 'local-dev') {
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

function proxyFetch(targetUrl: string, method: string, headers: Record<string, string> = {}, body?: string): Promise<any> {
  return api.post('/proxy', { url: targetUrl, method, headers, body })
}

function unpackRemote(data: any): any {
  if (data?.code === 200 || data?.code === 0) {
    return data.result || data.data || data
  }
  return data
}

const remoteAuthBase = 'https://auth.htui.cc/api/pas'

export interface RemoteLoginResult {
  token: string
  user?: any
}

export function remoteLogin(mobile: string, password: string): Promise<RemoteLoginResult> {
  const reqBody = JSON.stringify({ mobile, password })
  console.log('[LOGIN] request =>', JSON.stringify({ url: `${remoteAuthBase}/user/login`, body: { mobile, password } }))
  return proxyFetch(`${remoteAuthBase}/user/login`, 'POST', { 'Content-Type': 'application/json' }, reqBody).then((data) => {
    console.log('[LOGIN] proxy unpacked data =>', JSON.stringify(data))
    const result = unpackRemote(data)
    if (result?.token) {
      return result
    }
    return Promise.reject(new Error(data?.message || result?.message || '登录失败'))
  }).catch((err) => {
    console.error('[LOGIN] error =>', err?.message, err)
    return Promise.reject(err)
  })
}

export function remoteRegister(mobile: string, password: string, name?: string): Promise<any> {
  const params: Record<string, any> = { mobile, password }
  if (name) params.name = name
  return proxyFetch(`${remoteAuthBase}/user/register`, 'POST', { 'Content-Type': 'application/json' }, JSON.stringify(params)).then((data) => {
    const result = unpackRemote(data)
    if (result) return result
    return Promise.reject(new Error(data?.message || '注册失败'))
  })
}

export function remoteUpdateProfile(token: string, name: string, avatar?: string): Promise<any> {
  const params: Record<string, any> = { name }
  if (avatar) params.avatar = avatar
  return proxyFetch(`${remoteAuthBase}/user/profile`, 'PUT', { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` }, JSON.stringify(params)).then((data) => {
    const result = unpackRemote(data)
    if (result) return result
    return Promise.reject(new Error(data?.message || '修改失败'))
  })
}

export function remoteGetProfile(token: string): Promise<any> {
  return proxyFetch(`${remoteAuthBase}/user/profile`, 'GET', { 'Authorization': `Bearer ${token}` }).then((data) => {
    const result = unpackRemote(data)
    if (result) return result
    return Promise.reject(new Error(data?.message || '获取失败'))
  })
}

export function remoteChangePassword(token: string, oldPassword: string, newPassword: string): Promise<any> {
  return proxyFetch(`${remoteAuthBase}/user/change-password`, 'POST', { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` }, JSON.stringify({ oldPassword, newPassword })).then((data) => {
    const result = unpackRemote(data)
    if (result) return result
    return Promise.reject(new Error(data?.message || '修改失败'))
  })
}

export default api
