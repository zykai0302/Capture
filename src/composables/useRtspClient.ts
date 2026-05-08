import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { RtspClientStatus } from '../types'

const INVOKE_TIMEOUT = 30000

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out after ${INVOKE_TIMEOUT / 1000}s`)), INVOKE_TIMEOUT)
    ),
  ])
}

export function useRtspClient() {
  const streams = ref<Record<string, RtspClientStatus>>({})
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function connect(name: string, url: string, protocol: string, username?: string, password?: string) {
    try {
      loading.value = true
      error.value = null
      const streamId = await invokeWithTimeout<string>('rtsp_client_connect', {
        name,
        url,
        protocol,
        username: username ?? null,
        password: password ?? null,
      })
      await refreshStatus()
      return streamId
    } catch (e: any) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function disconnect(streamId: string) {
    try {
      loading.value = true
      error.value = null
      await invokeWithTimeout('rtsp_client_disconnect', { streamId })
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function refreshStatus() {
    loading.value = true
    try {
      const statuses = await invoke<RtspClientStatus[]>('rtsp_client_status')
      const map: Record<string, RtspClientStatus> = {}
      for (const s of statuses) {
        map[s.stream_id] = s
      }
      streams.value = map
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  function getStatus(streamId: string): RtspClientStatus | undefined {
    return streams.value[streamId]
  }

  function isConnected(streamId: string): boolean {
    const status = streams.value[streamId]
    if (!status) return false
    return status.state === 'Connected'
  }

  function getPreviewUrl(streamId: string, port: number): string {
    return `http://127.0.0.1:${port}/${streamId}`
  }

  let refreshInterval: ReturnType<typeof setInterval> | null = null

  onMounted(async () => {
    await refreshStatus()
    refreshInterval = setInterval(refreshStatus, 3000)
  })

  onUnmounted(() => {
    if (refreshInterval) clearInterval(refreshInterval)
  })

  return {
    streams,
    loading,
    error,
    connect,
    disconnect,
    refreshStatus,
    getStatus,
    isConnected,
    getPreviewUrl,
  }
}
