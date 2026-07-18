import { getApiBase } from '@/api/index'
import { useAuthStore } from '@/stores/auth'

export interface SSEOptions {
  url: string
  headers?: Record<string, string>
  onEvent?: (event: any) => void
  onError?: (err: Error) => void
  onDone?: () => void
}

export function connectSSE(options: SSEOptions): { close: () => void } {
  const ctrl = new AbortController()
  const auth = useAuthStore()
  const headers: Record<string, string> = {
    'Accept': 'text/event-stream',
    ...(options.headers || {}),
  }
  if (auth.token) {
    headers['Authorization'] = auth.token
  }

  fetch(options.url, {
    method: 'GET',
    headers,
    signal: ctrl.signal,
  })
    .then(async (response) => {
      if (!response.ok) {
        options.onError?.(new Error(`HTTP ${response.status}`))
        options.onDone?.()
        return
      }

      const reader = response.body?.getReader()
      if (!reader) {
        options.onError?.(new Error('No response body'))
        options.onDone?.()
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
              options.onEvent?.(event)
            } catch {
              // ignore
            }
            currentData = ''
          }
        }
      }
      options.onDone?.()
    })
    .catch((err) => {
      if (err.name !== 'AbortError') {
        options.onError?.(err)
      }
      options.onDone?.()
    })

  return {
    close: () => ctrl.abort(),
  }
}

export function getTaskSSEUrl(): string {
  return `${getApiBase()}/task/progress/stream`
}
