import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { CaptureSource, CaptureSourceList } from '../types'

export function useSources() {
  const monitors = ref<CaptureSource[]>([])
  const windows = ref<CaptureSource[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const thumbnailMap = ref<Record<string, string>>({})

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

  async function fetchThumbnails() {
    const sources = [...monitors.value, ...windows.value]
    const results = await Promise.allSettled(
      sources.map(async (source) => {
        try {
          const base64Str = await invoke<string>('capture_thumbnail', {
            sourceId: source.id,
            sourceType: source.source_type,
            width: 320,
            height: 180,
            handle: source.handle,
          })
          return { id: source.id, url: `data:image/jpeg;base64,${base64Str}` }
        } catch (e) {
          console.error(`Thumbnail failed for ${source.id}:`, e)
          throw e
        }
      })
    )
    const newMap: Record<string, string> = {}
    for (const result of results) {
      if (result.status === 'fulfilled') {
        newMap[result.value.id] = result.value.url
      }
    }
    thumbnailMap.value = newMap
  }

  const allSources = computed(() => [...monitors.value, ...windows.value])

  let unlistenAdded: UnlistenFn | null = null
  let unlistenRemoved: UnlistenFn | null = null
  let refreshInterval: ReturnType<typeof setInterval> | null = null

  onMounted(async () => {
    await refresh()
    fetchThumbnails()

    unlistenAdded = await listen('source-added', () => {
      refresh().then(() => {
        fetchThumbnails()
      })
    })
    unlistenRemoved = await listen('source-removed', () => {
      refresh().then(() => {
        fetchThumbnails()
      })
    })

    refreshInterval = setInterval(() => {
      refresh().then(() => {
        fetchThumbnails()
      })
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
    thumbnailMap,
  }
}
