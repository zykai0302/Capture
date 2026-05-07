import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PipelineStatus } from '../types'

const INVOKE_TIMEOUT = 30000

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out after ${INVOKE_TIMEOUT / 1000}s`)), INVOKE_TIMEOUT)
    ),
  ])
}

export function usePipeline() {
  // Use a plain reactive object instead of Map for better Vue reactivity
  const pipelineMap = ref<Record<string, PipelineStatus>>({})
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function startStream(source: { id: string; source_type: string; name: string; width: number; height: number; x: number; y: number; handle: number }) {
    try {
      await invokeWithTimeout('start_stream', {
        sourceId: source.id,
        sourceType: source.source_type,
        sourceName: source.name,
        width: source.width,
        height: source.height,
        x: source.x,
        y: source.y,
        handle: source.handle,
      })
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
      throw e
    }
  }

  async function stopStream(sourceId: string) {
    try {
      await invokeWithTimeout('stop_stream', { sourceId })
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
      throw e
    }
  }

  async function stopAllStreams() {
    try {
      await invokeWithTimeout('stop_all_streams', undefined)
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function refreshStatus() {
    loading.value = true
    try {
      const statuses = await invoke<PipelineStatus[]>('get_pipeline_status')
      const map: Record<string, PipelineStatus> = {}
      for (const s of statuses) {
        map[s.source_id] = s
      }
      pipelineMap.value = map
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  function getStatus(sourceId: string): PipelineStatus | undefined {
    return pipelineMap.value[sourceId]
  }

  function isStreaming(sourceId: string): boolean {
    const status = pipelineMap.value[sourceId]
    if (!status) return false
    return status.state === 'Running'
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
    pipelines: pipelineMap,
    loading,
    error,
    startStream,
    stopStream,
    stopAllStreams,
    refreshStatus,
    getStatus,
    isStreaming,
  }
}
