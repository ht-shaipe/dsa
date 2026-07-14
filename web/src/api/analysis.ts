import { callApi, callApiWithTimeout, getApiBase } from './index'
import { useAuthStore } from '@/stores/auth'

function getApiBaseUrl(): string {
  if (typeof window !== 'undefined' && ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)) {
    return 'http://127.0.0.1:18080/api/v1'
  }
  return '/api/v1'
}

export interface AnalysisStreamCallbacks {
  onStatus?: (content: string) => void
  onText?: (chunk: string) => void
  onReport?: (data: any) => void
  onError?: (message: string) => void
  onDone?: () => void
}

export function analyzeStream(code: string, name: string, callbacks: AnalysisStreamCallbacks): AbortController {
  const ctrl = new AbortController()
  const baseUrl = getApiBaseUrl()
  const url = `${baseUrl}/analysis/stream`
  const auth = useAuthStore()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream',
  }
  if (auth.token) {
    headers['Authorization'] = `Bearer ${auth.token}`
  }

  fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify({ code, name }),
    signal: ctrl.signal,
  })
    .then(async (response) => {
      if (!response.ok) {
        const text = await response.text()
        try {
          const err = JSON.parse(text)
          callbacks.onError?.(err.message || `HTTP ${response.status}`)
        } catch {
          callbacks.onError?.(`HTTP ${response.status}`)
        }
        callbacks.onDone?.()
        return
      }

      const reader = response.body?.getReader()
      if (!reader) {
        callbacks.onError?.('No response body')
        callbacks.onDone?.()
        return
      }

      const decoder = new TextDecoder()
      let buffer = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })
        const lines = buffer.split('\n')
        buffer = lines.pop() || ''

        let currentData = ''
        for (const line of lines) {
          if (line.startsWith('data: ')) {
            currentData = line.slice(6)
          } else if (line.startsWith('event: ')) {
            // next event type
          } else if (line === '' && currentData) {
            try {
              const event = JSON.parse(currentData)
              switch (event.type) {
                case 'status':
                  callbacks.onStatus?.(event.content)
                  break
                case 'text':
                  callbacks.onText?.(event.content)
                  break
                case 'report':
                  callbacks.onReport?.(event)
                  break
                case 'raw':
                  callbacks.onReport?.(event)
                  break
                case 'complete':
                  break
                case 'error':
                  callbacks.onError?.(event.content)
                  break
              }
            } catch {
              // ignore parse errors for non-JSON data
            }
            currentData = ''
          }
        }
      }
      callbacks.onDone?.()
    })
    .catch((err) => {
      if (err.name !== 'AbortError') {
        callbacks.onError?.(err.message || '网络请求失败')
      }
      callbacks.onDone?.()
    })

  return ctrl
}

export const analysisApi = {
  analyze: (code: string, name?: string) => callApiWithTimeout('analysis', 'analyze', { code, name }, 180000),
  analyzeStream,
  batch: (codes: string) => callApiWithTimeout('analysis', 'batch', { codes }, 300000),
  report: (params: { id?: number; queryId?: string }) => callApi('analysis', 'report', params),
  list: (params?: { code?: string; limit?: number }) => callApi('analysis', 'list', params || {}),
  marketReview: (params?: Record<string, any>) => callApiWithTimeout('analysis', 'market-review', params || {}, 300000),
  historyList: (params?: { code?: string; limit?: number; offset?: number }) => callApi('analysis', 'history_list', params || {}),
  historyDetail: (id: number) => callApi('analysis', 'history_detail', { id }),
  historySearch: (keyword: string, limit?: number) => callApi('analysis', 'history_search', { keyword, limit: limit || 20 }),
  historyCompare: (id1: number, id2: number) => callApi('analysis', 'history_compare', { id1, id2 }),
}
