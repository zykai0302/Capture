import { describe, it, expect, vi, beforeEach } from 'vitest'
import { SourceType, Codec, EncodeMode, RateControl, EncodePreset, PipelineStateEnum } from '../types'
import type { PipelineStatus, CaptureSourceList, AppConfig, GpuCapability, RemoteStatus } from '../types'

// Mock @tauri-apps/api/core - factory must be self-contained (hoisted)
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}))

// Mock Vue lifecycle hooks
vi.mock('vue', async () => {
  const actual = await vi.importActual<typeof import('vue')>('vue')
  return {
    ...actual,
    onMounted: vi.fn((fn: () => void) => fn()),
    onUnmounted: vi.fn(),
  }
})

// Import after mocks (they are hoisted above imports)
import { usePipeline } from '../composables/usePipeline'
import { useSources } from '../composables/useSources'
import { useConfig } from '../composables/useConfig'
import { useRemoteControl } from '../composables/useRemoteControl'

// Get the mocked invoke after hoisting
const invokeMock = vi.mocked(await import('@tauri-apps/api/core')).invoke

beforeEach(() => {
  invokeMock.mockReset()
})

describe('usePipeline', () => {
  it('initializes with empty pipeline map', async () => {
    invokeMock.mockResolvedValue([])
    const { pipelines, loading, error } = usePipeline()
    await vi.waitFor(() => expect(loading.value).toBe(false))
    expect(Object.keys(pipelines.value).length).toBe(0)
    expect(error.value).toBeNull()
  })

  it('refreshStatus populates pipeline map', async () => {
    const mockStatuses: PipelineStatus[] = [
      {
        source_id: 'screen-0', state: PipelineStateEnum.Running, encoder_used: 'amfh264enc',
        is_gpu: true, fps: 30, bitrate_kbps: 4000, latency_ms: 15,
        rtsp_url: 'rtsp://127.0.0.1:8554/screen-0',
      },
    ]
    invokeMock.mockResolvedValue(mockStatuses)

    const { refreshStatus, pipelines } = usePipeline()
    await refreshStatus()

    expect(invokeMock).toHaveBeenCalledWith('get_pipeline_status')
    expect(Object.keys(pipelines.value).length).toBe(1)
    expect(pipelines.value['screen-0']?.state).toBe(PipelineStateEnum.Running)
  })

  it('startStream calls invoke and refreshes', async () => {
    invokeMock.mockResolvedValue(undefined)

    const { startStream } = usePipeline()
    await startStream({ id: 'screen-0', source_type: SourceType.Monitor, name: '显示器 0', width: 1920, height: 1080 })

    expect(invokeMock).toHaveBeenCalledWith('start_stream', {
      sourceId: 'screen-0',
      sourceType: 'Monitor',
      sourceName: '显示器 0',
      width: 1920,
      height: 1080,
    })
  })

  it('stopStream calls invoke', async () => {
    invokeMock.mockResolvedValue(undefined)

    const { stopStream } = usePipeline()
    await stopStream('screen-0')

    expect(invokeMock).toHaveBeenCalledWith('stop_stream', { sourceId: 'screen-0' })
  })

  it('stopAllStreams calls invoke', async () => {
    invokeMock.mockResolvedValue(undefined)

    const { stopAllStreams } = usePipeline()
    await stopAllStreams()

    expect(invokeMock).toHaveBeenCalledWith('stop_all_streams', undefined)
  })

  it('sets error on invoke failure', async () => {
    invokeMock.mockRejectedValue('Pipeline error')

    const { startStream, error } = usePipeline()
    try { await startStream({ id: 'screen-0', source_type: SourceType.Monitor, name: '显示器 0', width: 1920, height: 1080 }) } catch {}

    expect(error.value).toBe('Pipeline error')
  })

  it('isStreaming returns true for Running pipeline', async () => {
    invokeMock.mockResolvedValue([
      { source_id: 'screen-0', state: PipelineStateEnum.Running, encoder_used: 'x264enc', is_gpu: false, fps: 30, bitrate_kbps: 4000, latency_ms: 20, rtsp_url: 'rtsp://localhost:8554/screen-0' },
    ])

    const { refreshStatus, isStreaming } = usePipeline()
    await refreshStatus()

    expect(isStreaming('screen-0')).toBe(true)
    expect(isStreaming('screen-1')).toBe(false)
  })

  it('getStatus returns pipeline for given sourceId', async () => {
    const mockStatus: PipelineStatus = {
      source_id: 'screen-0', state: PipelineStateEnum.Running, encoder_used: 'amfh264enc',
      is_gpu: true, fps: 30, bitrate_kbps: 4000, latency_ms: 15,
      rtsp_url: 'rtsp://127.0.0.1:8554/screen-0',
    }
    invokeMock.mockResolvedValue([mockStatus])

    const { refreshStatus, getStatus } = usePipeline()
    await refreshStatus()

    expect(getStatus('screen-0')).toEqual(mockStatus)
    expect(getStatus('nonexistent')).toBeUndefined()
  })
})

describe('useSources', () => {
  it('initializes with empty arrays', async () => {
    invokeMock.mockResolvedValue({ monitors: [], windows: [] })
    const { monitors, windows, loading, error } = useSources()
    await vi.waitFor(() => expect(loading.value).toBe(false))
    expect(monitors.value).toEqual([])
    expect(windows.value).toEqual([])
    expect(error.value).toBeNull()
  })

  it('refresh populates monitors and windows', async () => {
    const mockList: CaptureSourceList = {
      monitors: [
        { id: 'screen-0', name: '主显示器', source_type: SourceType.Monitor, width: 1920, height: 1080, is_streaming: false, rtsp_url: null },
      ],
      windows: [
        { id: 'window-1', name: '记事本', source_type: SourceType.Window, width: 800, height: 600, is_streaming: false, rtsp_url: null },
      ],
    }
    invokeMock.mockResolvedValue(mockList)

    const { refresh, monitors, windows } = useSources()
    await refresh()

    expect(invokeMock).toHaveBeenCalledWith('list_sources')
    expect(monitors.value).toHaveLength(1)
    expect(windows.value).toHaveLength(1)
  })

  it('sets error on invoke failure', async () => {
    invokeMock.mockRejectedValue('Source enumeration failed')

    const { refresh, error } = useSources()
    await refresh()

    expect(error.value).toBe('Source enumeration failed')
  })
})

describe('useConfig', () => {
  it('initializes with null config and gpuCaps', async () => {
    invokeMock.mockResolvedValue(null)
    const { config, gpuCaps, loading, error } = useConfig()
    await vi.waitFor(() => expect(loading.value).toBe(false))
    expect(config.value).toBeNull()
    expect(gpuCaps.value).toBeNull()
    expect(error.value).toBeNull()
  })

  it('loadConfig populates config', async () => {
    const mockConfig: AppConfig = {
      rtsp_port: 8554, rtsp_max_clients: 10, ws_port: 9001,
      ws_password: '', auto_reconnect: true,
      default_encode: {
        codec: Codec.H264, mode: EncodeMode.Auto, resolution: { Original: true },
        framerate: 30, bitrate_kbps: 4000, max_bitrate_kbps: 6000,
        rate_control: RateControl.VBR, gop_size: 30, preset: EncodePreset.Balanced,
      },
    }
    invokeMock.mockResolvedValue(mockConfig)

    const { loadConfig, config } = useConfig()
    await loadConfig()

    expect(invokeMock).toHaveBeenCalledWith('get_config')
    expect(config.value?.rtsp_port).toBe(8554)
  })

  it('loadGpuCapabilities populates gpuCaps', async () => {
    const mockGpu: GpuCapability = {
      has_amf: true, has_mf: false, has_videotoolbox: false, has_vaapi: false,
      amf_encoders: ['amfh264enc'], mf_encoders: [], vt_encoders: [], vaapi_encoders: [],
    }
    invokeMock.mockResolvedValue(mockGpu)

    const { loadGpuCapabilities, gpuCaps } = useConfig()
    await loadGpuCapabilities()

    expect(invokeMock).toHaveBeenCalledWith('get_gpu_capabilities')
    expect(gpuCaps.value?.has_amf).toBe(true)
  })

  it('getAvailableEncoders returns encoder list', async () => {
    invokeMock.mockResolvedValue(['amfh264enc', 'x264enc'])

    const { getAvailableEncoders } = useConfig()
    const encoders = await getAvailableEncoders()

    expect(invokeMock).toHaveBeenCalledWith('get_available_encoders')
    expect(encoders).toEqual(['amfh264enc', 'x264enc'])
  })

  it('getAvailableEncoders returns empty on error', async () => {
    invokeMock.mockRejectedValue('No encoders')

    const { getAvailableEncoders, error } = useConfig()
    const encoders = await getAvailableEncoders()

    expect(encoders).toEqual([])
    expect(error.value).toBe('No encoders')
  })
})

describe('useRemoteControl', () => {
  it('initializes with default status', () => {
    const { status, loading, error } = useRemoteControl()
    expect(status.value.is_running).toBe(false)
    expect(status.value.ws_port).toBe(9001)
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('refresh updates status from backend', async () => {
    const mockStatus: RemoteStatus = {
      ws_port: 9002, is_running: true, client_count: 2, mouse_enabled: true, keyboard_enabled: false,
    }
    invokeMock.mockResolvedValue(mockStatus)

    const { refresh, status } = useRemoteControl()
    await refresh()

    expect(invokeMock).toHaveBeenCalledWith('get_remote_status')
    expect(status.value.is_running).toBe(true)
    expect(status.value.client_count).toBe(2)
  })

  it('start calls invoke with params', async () => {
    invokeMock.mockResolvedValue(undefined)

    const { start } = useRemoteControl()
    await start(9002, 'password123')

    expect(invokeMock).toHaveBeenCalledWith('start_remote_control', {
      port: 9002, password: 'password123',
    })
  })

  it('stop calls invoke', async () => {
    invokeMock.mockResolvedValue(undefined)

    const { stop } = useRemoteControl()
    await stop()

    expect(invokeMock).toHaveBeenCalledWith('stop_remote_control')
  })

  it('sets error on failure', async () => {
    invokeMock.mockRejectedValue('WS failed')

    const { start, error } = useRemoteControl()
    await start()

    expect(error.value).toBe('WS failed')
  })
})
