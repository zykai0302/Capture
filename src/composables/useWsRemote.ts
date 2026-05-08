import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { WsClientStatus } from '../types'
import { MouseButton, ClickAction } from '../types'

const INVOKE_TIMEOUT = 30000

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out after ${INVOKE_TIMEOUT / 1000}s`)), INVOKE_TIMEOUT)
    ),
  ])
}

export function useWsRemote() {
  const status = ref<WsClientStatus>({
    is_connected: false,
    is_reconnecting: false,
    reconnect_attempt: 0,
    max_reconnect_attempts: 3,
    remote_url: '',
  })
  const loading = ref(false)
  const error = ref<string | null>(null)
  const controlEnabled = ref(false)

  async function connect(url: string, password?: string) {
    try {
      loading.value = true
      error.value = null
      await invokeWithTimeout('ws_remote_connect', {
        url,
        password: password ?? null,
      })
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function disconnect() {
    try {
      loading.value = true
      error.value = null
      await invokeWithTimeout('ws_remote_disconnect')
      await refreshStatus()
    } catch (e: any) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function refreshStatus() {
    try {
      const s = await invoke<WsClientStatus>('ws_remote_status')
      status.value = s
      controlEnabled.value = s.is_connected
    } catch (e: any) {
      error.value = String(e)
    }
  }

  // Mouse control methods — send ClientRemoteCommand via ws_remote_send_command
  async function sendMouseMove(streamId: string, relX: number, relY: number) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'mouse_move',
          stream_id: streamId,
          data: { rel_x: relX, rel_y: relY },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function sendMouseClick(streamId: string, relX: number, relY: number, button: MouseButton, action: ClickAction) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'mouse_click',
          stream_id: streamId,
          data: { rel_x: relX, rel_y: relY, button, action },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function sendMouseScroll(streamId: string, relX: number, relY: number, dx: number, dy: number) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'mouse_scroll',
          stream_id: streamId,
          data: { rel_x: relX, rel_y: relY, dx, dy },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function sendMouseDrag(streamId: string, fromRelX: number, fromRelY: number, toRelX: number, toRelY: number, button: MouseButton) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'mouse_drag',
          stream_id: streamId,
          data: { from_rel_x: fromRelX, from_rel_y: fromRelY, to_rel_x: toRelX, to_rel_y: toRelY, button },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  // Keyboard control methods
  async function sendKeyPress(streamId: string, key: string, modifiers: string[] = []) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'key_press',
          stream_id: streamId,
          data: { key, modifiers },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function sendKeyCombo(streamId: string, keys: string[]) {
    try {
      await invokeWithTimeout('ws_remote_send_command', {
        command: {
          type: 'key_combo',
          stream_id: streamId,
          data: { keys },
        },
      })
    } catch (e: any) {
      error.value = String(e)
    }
  }

  async function setResolution(width: number, height: number) {
    try {
      await invokeWithTimeout('ws_remote_set_resolution', { width, height })
    } catch (e: any) {
      error.value = String(e)
    }
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
    status,
    loading,
    error,
    controlEnabled,
    connect,
    disconnect,
    refreshStatus,
    sendMouseMove,
    sendMouseClick,
    sendMouseScroll,
    sendMouseDrag,
    sendKeyPress,
    sendKeyCombo,
    setResolution,
  }
}
