import { describe, it, expect } from 'vitest'
import {
  SourceType,
  Codec,
  EncodeMode,
  RateControl,
  EncodePreset,
  PipelineStateEnum,
  defaultEncodeConfig,
  isPipelineRunning,
  isPipelineError,
  getPipelineError,
  getPipelineStateLabel,
} from '../types'
import type {
  CaptureSource,
  CaptureSourceList,
  Resolution,
  PipelineStatus,
  GpuCapability,
  AppConfig,
  RemoteStatus,
} from '../types'

describe('SourceType', () => {
  it('has Monitor and Window values', () => {
    expect(SourceType.Monitor).toBe('Monitor')
    expect(SourceType.Window).toBe('Window')
  })
})

describe('Codec', () => {
  it('has H264 and H265 values', () => {
    expect(Codec.H264).toBe('H264')
    expect(Codec.H265).toBe('H265')
  })
})

describe('EncodeMode', () => {
  it('has Auto, GpuOnly, CpuOnly values', () => {
    expect(EncodeMode.Auto).toBe('Auto')
    expect(EncodeMode.GpuOnly).toBe('GpuOnly')
    expect(EncodeMode.CpuOnly).toBe('CpuOnly')
  })
})

describe('RateControl', () => {
  it('has CBR and VBR values', () => {
    expect(RateControl.CBR).toBe('CBR')
    expect(RateControl.VBR).toBe('VBR')
  })
})

describe('EncodePreset', () => {
  it('has Speed, Balanced, Quality values', () => {
    expect(EncodePreset.Speed).toBe('Speed')
    expect(EncodePreset.Balanced).toBe('Balanced')
    expect(EncodePreset.Quality).toBe('Quality')
  })
})

describe('defaultEncodeConfig', () => {
  it('returns default values matching Rust backend', () => {
    const config = defaultEncodeConfig()
    expect(config.codec).toBe('H264')
    expect(config.mode).toBe('Auto')
    expect(config.resolution).toEqual({ Original: true })
    expect(config.framerate).toBe(30)
    expect(config.bitrate_kbps).toBe(4000)
    expect(config.max_bitrate_kbps).toBe(6000)
    expect(config.rate_control).toBe('VBR')
    expect(config.gop_size).toBe(30)
    expect(config.preset).toBe('Balanced')
  })

  it('returns a new object each time', () => {
    const a = defaultEncodeConfig()
    const b = defaultEncodeConfig()
    expect(a).not.toBe(b)
    expect(a).toEqual(b)
  })
})

describe('Resolution', () => {
  it('Original resolution is { Original: true }', () => {
    const res: Resolution = { Original: true }
    expect(res.Original).toBe(true)
  })

  it('Custom resolution has width and height', () => {
    const res: Resolution = { Custom: { width: 1920, height: 1080 } }
    expect(res.Custom?.width).toBe(1920)
    expect(res.Custom?.height).toBe(1080)
  })
})

describe('isPipelineRunning', () => {
  it('returns true for Running state', () => {
    expect(isPipelineRunning(PipelineStateEnum.Running)).toBe(true)
  })

  it('returns false for Stopped state', () => {
    expect(isPipelineRunning(PipelineStateEnum.Stopped)).toBe(false)
  })

  it('returns false for Starting state', () => {
    expect(isPipelineRunning(PipelineStateEnum.Starting)).toBe(false)
  })

  it('returns false for Error state', () => {
    expect(isPipelineRunning({ Error: 'something' })).toBe(false)
  })
})

describe('isPipelineError', () => {
  it('returns true for Error state', () => {
    expect(isPipelineError({ Error: 'crashed' })).toBe(true)
  })

  it('returns false for Running state', () => {
    expect(isPipelineError(PipelineStateEnum.Running)).toBe(false)
  })

  it('returns false for Stopped state', () => {
    expect(isPipelineError(PipelineStateEnum.Stopped)).toBe(false)
  })
})

describe('getPipelineError', () => {
  it('returns error message for Error state', () => {
    expect(getPipelineError({ Error: 'encoder failed' })).toBe('encoder failed')
  })

  it('returns empty string for non-error states', () => {
    expect(getPipelineError(PipelineStateEnum.Running)).toBe('')
    expect(getPipelineError(PipelineStateEnum.Stopped)).toBe('')
    expect(getPipelineError(PipelineStateEnum.Starting)).toBe('')
  })
})

describe('getPipelineStateLabel', () => {
  it('returns Chinese labels for string states', () => {
    expect(getPipelineStateLabel(PipelineStateEnum.Stopped)).toBe('已停止')
    expect(getPipelineStateLabel(PipelineStateEnum.Starting)).toBe('启动中')
    expect(getPipelineStateLabel(PipelineStateEnum.Running)).toBe('推流中')
  })

  it('returns error message for Error state', () => {
    expect(getPipelineStateLabel({ Error: 'GPU crash' })).toBe('GPU crash')
  })
})

describe('CaptureSource', () => {
  it('has required fields', () => {
    const source: CaptureSource = {
      id: 'screen-0',
      name: '主显示器',
      source_type: SourceType.Monitor,
      width: 1920,
      height: 1080,
      x: 0,
      y: 0,
      is_streaming: false,
      rtsp_url: null,
      handle: 0,
    }
    expect(source.id).toBe('screen-0')
    expect(source.source_type).toBe('Monitor')
    expect(source.rtsp_url).toBeNull()
  })
})

describe('CaptureSourceList', () => {
  it('has monitors and windows arrays', () => {
    const list: CaptureSourceList = {
      monitors: [{ id: 'screen-0', name: 'M', source_type: SourceType.Monitor, width: 1920, height: 1080, x: 0, y: 0, is_streaming: false, rtsp_url: null, handle: 0 }],
      windows: [{ id: 'window-1', name: 'W', source_type: SourceType.Window, width: 800, height: 600, x: 100, y: 100, is_streaming: false, rtsp_url: null, handle: 0 }],
    }
    expect(list.monitors).toHaveLength(1)
    expect(list.windows).toHaveLength(1)
  })
})

describe('PipelineStatus', () => {
  it('has required fields', () => {
    const status: PipelineStatus = {
      source_id: 'screen-0',
      state: PipelineStateEnum.Running,
      encoder_used: 'amfh264enc',
      is_gpu: true,
      fps: 30.0,
      bitrate_kbps: 4000,
      latency_ms: 15,
      rtsp_url: 'rtsp://127.0.0.1:8554/screen-0',
    }
    expect(status.is_gpu).toBe(true)
    expect(status.fps).toBe(30.0)
  })
})

describe('GpuCapability', () => {
  it('has required fields', () => {
    const cap: GpuCapability = {
      has_amf: true,
      has_mf: false,
      has_videotoolbox: false,
      has_vaapi: false,
      amf_encoders: ['amfh264enc'],
      mf_encoders: [],
      vt_encoders: [],
      vaapi_encoders: [],
    }
    expect(cap.has_amf).toBe(true)
    expect(cap.amf_encoders).toHaveLength(1)
  })
})

describe('AppConfig', () => {
  it('has required fields', () => {
    const config: AppConfig = {
      rtsp_port: 8554,
      rtsp_max_clients: 10,
      ws_port: 9001,
      ws_password: '',
      auto_reconnect: true,
      default_encode: defaultEncodeConfig(),
      preview_http_port: 8090,
    }
    expect(config.rtsp_port).toBe(8554)
    expect(config.auto_reconnect).toBe(true)
  })
})

describe('RemoteStatus', () => {
  it('has required fields', () => {
    const status: RemoteStatus = {
      ws_port: 9001,
      is_running: false,
      client_count: 0,
      mouse_enabled: true,
      keyboard_enabled: true,
    }
    expect(status.ws_port).toBe(9001)
    expect(status.is_running).toBe(false)
  })
})
