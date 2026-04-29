import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppConfig, GpuCapability } from '../types'

export function useConfig() {
  const config = ref<AppConfig | null>(null)
  const gpuCaps = ref<GpuCapability | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadConfig() {
    loading.value = true
    try {
      config.value = await invoke<AppConfig>('get_config')
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadGpuCapabilities() {
    try {
      gpuCaps.value = await invoke<GpuCapability>('get_gpu_capabilities')
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function getAvailableEncoders(): Promise<string[]> {
    try {
      return await invoke<string[]>('get_available_encoders')
    } catch (e: any) {
      error.value = String(e)
      return []
    }
  }

  onMounted(async () => {
    await Promise.all([loadConfig(), loadGpuCapabilities()])
  })

  return {
    config,
    gpuCaps,
    loading,
    error,
    loadConfig,
    loadGpuCapabilities,
    getAvailableEncoders,
  }
}
