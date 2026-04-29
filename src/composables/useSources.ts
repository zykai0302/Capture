import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { CaptureSource, CaptureSourceList } from '../types'

export function useSources() {
  const monitors = ref<CaptureSource[]>([])
  const windows = ref<CaptureSource[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const list = await invoke<CaptureSourceList>('list_sources')
      monitors.value = list.monitors
      windows.value = list.windows
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  const allSources = ref<CaptureSource[]>([])

  function updateAllSources() {
    allSources.value = [...monitors.value, ...windows.value]
  }

  let unlistenAdded: UnlistenFn | null = null
  let unlistenRemoved: UnlistenFn | null = null
  let refreshInterval: ReturnType<typeof setInterval> | null = null

  onMounted(async () => {
    await refresh()
    updateAllSources()

    unlistenAdded = await listen('source-added', () => {
      refresh().then(updateAllSources)
    })
    unlistenRemoved = await listen('source-removed', () => {
      refresh().then(updateAllSources)
    })

    refreshInterval = setInterval(() => {
      refresh().then(updateAllSources)
    }, 5000)
  })

  onUnmounted(() => {
    unlistenAdded?.()
    unlistenRemoved?.()
    if (refreshInterval) clearInterval(refreshInterval)
  })

  return {
    monitors,
    windows,
    allSources,
    loading,
    error,
    refresh,
  }
}
