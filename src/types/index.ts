// === Backend Data Types (mirroring Rust structs) ===

export enum SourceType {
  Monitor = 'Monitor',
  Window = 'Window',
}

export interface CaptureSource {
  id: string
  name: string
  source_type: SourceType
  width: number
  height: number
  x: number
  y: number
  is_streaming: boolean
  rtsp_url: string | null
  handle: number
}

export interface CaptureSourceList {
  monitors: CaptureSource[]
  windows: CaptureSource[]
}

export enum Codec {
  H264 = 'H264',
  H265 = 'H265',
}

export enum EncodeMode {
  Auto = 'Auto',
  GpuOnly = 'GpuOnly',
  CpuOnly = 'CpuOnly',
}

export enum RateControl {
  CBR = 'CBR',
  VBR = 'VBR',
}

export enum EncodePreset {
  Speed = 'Speed',
  Balanced = 'Balanced',
  Quality = 'Quality',
}

export interface ResolutionOriginal {
  Original: true
}

export interface ResolutionCustom {
  Custom: { width: number; height: number }
}

export type Resolution = ResolutionOriginal | ResolutionCustom

export interface EncodeConfig {
  codec: Codec
  mode: EncodeMode
  resolution: Resolution
  framerate: number
  bitrate_kbps: number
  max_bitrate_kbps: number
  rate_control: RateControl
  gop_size: number
  preset: EncodePreset
}

export enum PipelineStateEnum {
  Stopped = 'Stopped',
  Starting = 'Starting',
  Running = 'Running',
}

export interface PipelineStateError {
  Error: string
}

export type PipelineState = PipelineStateEnum | PipelineStateError

export interface PipelineStatus {
  source_id: string
  state: PipelineState
  encoder_used: string
  is_gpu: boolean
  fps: number
  bitrate_kbps: number
  latency_ms: number
  rtsp_url: string
}

export interface GpuCapability {
  has_amf: boolean
  has_mf: boolean
  has_videotoolbox: boolean
  has_vaapi: boolean
  amf_encoders: string[]
  mf_encoders: string[]
  vt_encoders: string[]
  vaapi_encoders: string[]
}

export interface AppConfig {
  rtsp_port: number
  rtsp_max_clients: number
  ws_port: number
  ws_password: string
  auto_reconnect: boolean
  default_encode: EncodeConfig
  preview_http_port: number
}

export interface RemoteStatus {
  ws_port: number
  is_running: boolean
  client_count: number
  mouse_enabled: boolean
  keyboard_enabled: boolean
}

// === RTSP Client Types ===

export enum RtspClientStateEnum {
  Connecting = 'Connecting',
  Connected = 'Connected',
  Disconnected = 'Disconnected',
  Offline = 'Offline',
}

export interface RtspClientStateReconnecting {
  Reconnecting: { attempt: number; max_attempts: number }
}

export interface RtspClientStateError {
  Error: string
}

export type RtspClientState =
  | RtspClientStateEnum
  | RtspClientStateReconnecting
  | RtspClientStateError

export interface RtspClientStatus {
  stream_id: string
  name: string
  url: string
  state: RtspClientState
  resolution: [number, number] | null
  fps: number
  latency_ms: number
  protocol: string
}

// === WebSocket Remote Client Types ===

export interface WsClientStatus {
  is_connected: boolean
  is_reconnecting: boolean
  reconnect_attempt: number
  max_reconnect_attempts: number
  remote_url: string
}

// === Client Remote Commands (relative coordinates) ===

export enum MouseButton {
  Left = 'Left',
  Right = 'Right',
  Middle = 'Middle',
}

export enum ClickAction {
  Single = 'Single',
  Double = 'Double',
}

export interface RelativeMouseMoveData {
  rel_x: number
  rel_y: number
}

export interface RelativeMouseClickData {
  rel_x: number
  rel_y: number
  button: MouseButton
  action: ClickAction
}

export interface RelativeMouseScrollData {
  rel_x: number
  rel_y: number
  dx: number
  dy: number
}

export interface RelativeMouseDragData {
  from_rel_x: number
  from_rel_y: number
  to_rel_x: number
  to_rel_y: number
  button: MouseButton
}

// === Helper Functions ===

export function defaultEncodeConfig(): EncodeConfig {
  return {
    codec: Codec.H264,
    mode: EncodeMode.Auto,
    resolution: { Original: true },
    framerate: 30,
    bitrate_kbps: 4000,
    max_bitrate_kbps: 6000,
    rate_control: RateControl.VBR,
    gop_size: 30,
    preset: EncodePreset.Balanced,
  }
}

export function isPipelineRunning(state: PipelineState): boolean {
  if (typeof state === 'string') return state === PipelineStateEnum.Running
  return false
}

export function isPipelineError(state: PipelineState): boolean {
  return typeof state === 'object' && 'Error' in state
}

export function getPipelineError(state: PipelineState): string {
  if (typeof state === 'object' && 'Error' in state) return state.Error
  return ''
}

export function getPipelineStateLabel(state: PipelineState): string {
  if (typeof state === 'string') {
    switch (state) {
      case PipelineStateEnum.Stopped: return '已停止'
      case PipelineStateEnum.Starting: return '启动中'
      case PipelineStateEnum.Running: return '推流中'
      default: return state
    }
  }
  return state.Error
}

// === RTSP Client Helper Functions ===

export function isRtspClientConnected(state: RtspClientState): boolean {
  return state === RtspClientStateEnum.Connected
}

export function isRtspClientDisconnected(state: RtspClientState): boolean {
  return state === RtspClientStateEnum.Disconnected || state === RtspClientStateEnum.Offline
}

export function isRtspClientReconnecting(state: RtspClientState): boolean {
  return typeof state === 'object' && 'Reconnecting' in state
}

export function isRtspClientError(state: RtspClientState): boolean {
  return typeof state === 'object' && 'Error' in state
}

export function getRtspClientStateLabel(state: RtspClientState): string {
  if (typeof state === 'string') {
    switch (state) {
      case RtspClientStateEnum.Connecting: return '连接中'
      case RtspClientStateEnum.Connected: return '已连接'
      case RtspClientStateEnum.Disconnected: return '已断开'
      case RtspClientStateEnum.Offline: return '离线'
      default: return state
    }
  }
  if ('Reconnecting' in state) return `重连中 (${state.Reconnecting.attempt}/${state.Reconnecting.max_attempts})`
  return state.Error
}
