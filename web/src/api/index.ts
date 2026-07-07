import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import router from '@/router'
import { ElMessage } from 'element-plus'

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 60000,
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.request.use((config) => {
  const auth = useAuthStore()
  if (auth.token) {
    config.headers.Authorization = `Bearer ${auth.token}`
  }
  return config
})

api.interceptors.response.use(
  (res) => res.data,
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

/**
 * 统一API调用函数
 */
export function callApi(module: string, method: string, params: Record<string, any> = {}): Promise<any> {
  return api.post(`/${module}/${method}`, params)
}

export default api
