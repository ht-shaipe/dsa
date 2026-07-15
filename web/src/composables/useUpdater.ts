import { ref, computed } from 'vue'
import { invoke, Channel } from '@tauri-apps/api/core'

function isTauri(): boolean {
  return typeof window !== 'undefined' && !!((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)
}

export interface UpdateInfo {
  version: string
  currentVersion: string
  date?: string
  body?: string
}

export type UpdateStatus = 'unsupported' | 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error'

const status = ref<UpdateStatus>(isTauri() ? 'idle' : 'unsupported')
const updateInfo = ref<UpdateInfo | null>(null)
const progress = ref({ downloaded: 0, total: 0 })
const errorMessage = ref('')

export function useUpdater() {
  const hasUpdate = computed(() => status.value === 'available' || status.value === 'downloading' || status.value === 'ready')

  async function checkForUpdate(silent = false): Promise<boolean> {
    if (!isTauri()) {
      status.value = 'unsupported'
      return false
    }

    status.value = 'checking'
    errorMessage.value = ''

    try {
      const info: UpdateInfo | null = await invoke('check_update')
      if (info) {
        updateInfo.value = info
        status.value = 'available'
        return true
      } else {
        status.value = 'idle'
        return false
      }
    } catch (e: any) {
      errorMessage.value = e?.toString() || '检查更新失败'
      status.value = 'error'
      if (!silent) {
        console.warn('[Updater] check failed:', errorMessage.value)
      }
      return false
    }
  }

  async function downloadAndInstall(): Promise<boolean> {
    if (!isTauri() || !updateInfo.value) return false

    status.value = 'downloading'
    progress.value = { downloaded: 0, total: 0 }
    errorMessage.value = ''

    try {
      const onEvent = new Channel()
      onEvent.onmessage = (event: any) => {
        if (event.event === 'started') {
          progress.value.total = event.data?.contentLength || 0
        } else if (event.event === 'progress') {
          progress.value.downloaded += event.data?.chunkLength || 0
        } else if (event.event === 'finished') {
          status.value = 'ready'
        }
      }

      await invoke('install_update', { onEvent })
      status.value = 'ready'
      return true
    } catch (e: any) {
      errorMessage.value = e?.toString() || '下载更新失败'
      status.value = 'error'
      return false
    }
  }

  async function restartApp(): Promise<void> {
    if (!isTauri()) return
    try {
      const { relaunch } = await import('@tauri-apps/plugin-process')
      await relaunch()
    } catch {
      console.warn('[Updater] relaunch failed, trying window reload')
      window.location.reload()
    }
  }

  function reset() {
    status.value = isTauri() ? 'idle' : 'unsupported'
    updateInfo.value = null
    progress.value = { downloaded: 0, total: 0 }
    errorMessage.value = ''
  }

  return {
    status,
    updateInfo,
    progress,
    errorMessage,
    hasUpdate,
    isTauri: isTauri(),
    checkForUpdate,
    downloadAndInstall,
    restartApp,
    reset,
  }
}
