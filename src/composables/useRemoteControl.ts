import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { RemoteStatus } from '../types'

export function useRemoteControl() {
  const status = ref<RemoteStatus>({
    ws_port: 9001,
    is_running: false,
    client_count: 0,
    mouse_enabled: true,
    keyboard_enabled: true,
  })
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function start(port?: number, password?: string) {
    loading.value = true
    error.value = null
    try {
      await invoke('start_remote_control', { port: port ?? null, password: password ?? null })
      await refresh()
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function stop() {
    loading.value = true
    try {
      await invoke('stop_remote_control')
      await refresh()
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function refresh() {
    try {
      status.value = await invoke<RemoteStatus>('get_remote_status')
    } catch (e: any) {
      error.value = String(e)
    }
  }

  let refreshInterval: ReturnType<typeof setInterval> | null = null

  onMounted(async () => {
    await refresh()
    refreshInterval = setInterval(refresh, 5000)
  })

  onUnmounted(() => {
    if (refreshInterval) clearInterval(refreshInterval)
  })

  return {
    status,
    loading,
    error,
    start,
    stop,
    refresh,
  }
}
