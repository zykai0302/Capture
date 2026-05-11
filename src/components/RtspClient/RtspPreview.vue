<script setup lang="ts">
import { ref, computed, inject, watch } from 'vue'
import type { useRtspClient, useWsRemote, useConfig } from '../../composables'
import { isRtspClientConnected, isRtspClientDisconnected, isRtspClientReconnecting, isRtspClientError, MouseButton, ClickAction } from '../../types'

const props = defineProps<{
  streamId: string | null
}>()

const rtspClient = inject<ReturnType<typeof useRtspClient>>('rtspClient')!
const wsRemote = inject<ReturnType<typeof useWsRemote>>('wsRemote')!
const { config } = inject<ReturnType<typeof useConfig>>('config')!

const controlEnabled = ref(false)
const isDragging = ref(false)
const dragStartX = ref(0)
const dragStartY = ref(0)
let lastMoveTime = 0

const status = computed(() => {
  if (!props.streamId) return null
  return rtspClient.getStatus(props.streamId)
})

const isConnected = computed(() => status.value ? isRtspClientConnected(status.value.state) : false)
const isConnecting = computed(() => status.value?.state === 'Connecting')
const isDisconnected = computed(() => status.value ? isRtspClientDisconnected(status.value.state) : false)
const isReconnecting = computed(() => status.value ? isRtspClientReconnecting(status.value.state) : false)
const isError = computed(() => status.value ? isRtspClientError(status.value.state) : false)

const previewPort = computed(() => config.value?.preview_http_port ?? 8090)
const previewUrl = computed(() => {
  if (!props.streamId || !isConnected.value) return ''
  return rtspClient.getPreviewUrl(props.streamId, previewPort.value)
})

// Force re-render of img element when stream reconnects (key changes)
const imgKey = ref(0)
watch(previewUrl, (newUrl, oldUrl) => {
  if (newUrl && newUrl !== oldUrl) {
    imgKey.value++
  }
})

function getRelCoords(event: MouseEvent) {
  const rect = (event.target as HTMLElement).getBoundingClientRect()
  const relX = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width))
  const relY = Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height))
  return { relX, relY }
}

function onMouseMove(event: MouseEvent) {
  if (!controlEnabled.value || !wsRemote.status.value.is_connected || !props.streamId) return
  const now = Date.now()
  if (now - lastMoveTime < 33) return // ~30fps throttle
  lastMoveTime = now
  const { relX, relY } = getRelCoords(event)
  wsRemote.sendMouseMove(props.streamId, relX, relY)
}

function onMouseDown(event: MouseEvent) {
  if (!controlEnabled.value || !wsRemote.status.value.is_connected || !props.streamId) return
  if (event.button === 0) { // left button
    isDragging.value = true
    const coords = getRelCoords(event)
    dragStartX.value = coords.relX
    dragStartY.value = coords.relY
  }
  const { relX, relY } = getRelCoords(event)
  const button = event.button === 0 ? MouseButton.Left : event.button === 2 ? MouseButton.Right : MouseButton.Middle
  wsRemote.sendMouseClick(props.streamId, relX, relY, button, ClickAction.Single)
}

function onMouseUp(event: MouseEvent) {
  if (!controlEnabled.value || !wsRemote.status.value.is_connected || !props.streamId) return
  if (isDragging.value) {
    const endCoords = getRelCoords(event)
    wsRemote.sendMouseDrag(
      props.streamId,
      dragStartX.value, dragStartY.value,
      endCoords.relX, endCoords.relY,
      MouseButton.Left
    )
    isDragging.value = false
  }
}

function onWheel(event: WheelEvent) {
  if (!controlEnabled.value || !wsRemote.status.value.is_connected || !props.streamId) return
  const { relX, relY } = getRelCoords(event as any)
  wsRemote.sendMouseScroll(props.streamId, relX, relY, 0, event.deltaY > 0 ? -3 : 3)
}

function onImgError(event: Event) {
  const img = event.target as HTMLImageElement
  if (previewUrl.value) {
    setTimeout(() => { img.src = previewUrl.value + '?t=' + Date.now() }, 2000)
  }
}
</script>

<template>
  <div class="rtsp-preview">
    <div class="preview-header">
      <div class="preview-title-area">
        <span class="preview-title">{{ status?.name || 'RTSP 预览' }}</span>
        <span v-if="status" class="stream-url font-mono">{{ status.url }}</span>
      </div>
      <div class="preview-actions">
        <div class="control-toggle" v-if="isConnected">
          <span class="toggle-label">反控</span>
          <div class="toggle-switch" :class="{ active: controlEnabled }" @click="controlEnabled = !controlEnabled"></div>
        </div>
      </div>
    </div>

    <div class="preview-main">
      <div class="preview-container" :class="{ 'control-mode': controlEnabled }">
        <div class="preview-canvas" @contextmenu.prevent>
          <template v-if="isConnected && previewUrl">
            <img
              :key="imgKey"
              :src="previewUrl"
              class="preview-mjpeg"
              :style="{ cursor: controlEnabled ? 'crosshair' : 'default' }"
              @mousemove="onMouseMove"
              @mousedown="onMouseDown"
              @mouseup="onMouseUp"
              @wheel.prevent="onWheel"
              @error="onImgError"
              draggable="false"
            />
          </template>
          <template v-else-if="isConnecting">
            <div class="preview-starting">
              <div class="starting-spinner purple"></div>
              <div class="starting-text">连接 RTSP 流...</div>
              <div class="starting-sub">正在建立连接并协商媒体格式</div>
            </div>
          </template>
          <template v-else-if="isReconnecting">
            <div class="preview-starting">
              <div class="starting-spinner purple"></div>
              <div class="starting-text">重新连接中...</div>
            </div>
          </template>
          <template v-else-if="isError">
            <div class="preview-error">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.6">
                <circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
              </svg>
              <div class="error-text">连接失败</div>
              <div class="error-detail">{{ typeof status?.state === 'object' && 'Error' in (status?.state as any) ? (status?.state as any).Error : '' }}</div>
            </div>
          </template>
          <template v-else-if="isDisconnected">
            <div class="preview-error">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.6">
                <circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/>
              </svg>
              <div class="disconnected-text">{{ status?.state === 'Offline' ? '流源离线' : '连接已断开' }}</div>
              <div class="error-detail">服务器已停止推流，请点击左侧"重连"按钮重新连接</div>
            </div>
          </template>
          <template v-else>
            <div class="preview-placeholder">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
                <rect x="2" y="3" width="20" height="14" rx="2"/><rect x="7" y="21" width="10" height="2" rx="1"/>
              </svg>
              <div class="preview-placeholder-text">选择一个 RTSP 流</div>
              <div class="preview-placeholder-sub">从左侧列表添加或选择流</div>
            </div>
          </template>
        </div>

        <!-- HUD -->
        <template v-if="isConnected && status">
          <div class="preview-hud">
            <div class="hud-item" v-if="status.resolution">{{ status.resolution[0] }}x{{ status.resolution[1] }}</div>
            <div class="hud-item">{{ status.protocol.toUpperCase() }} · {{ status.latency_ms }}ms</div>
          </div>
          <div class="preview-hud-right">
            <div class="fps-counter">{{ status.fps.toFixed(0) }} FPS</div>
          </div>
          <div v-if="controlEnabled" class="control-indicator">
            <span class="control-dot"></span> 反控中
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rtsp-preview {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.preview-header {
  padding: 14px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
}

.preview-title-area {
  display: flex;
  align-items: center;
  gap: 10px;
}

.preview-title {
  font-size: 14px;
  font-weight: 600;
}

.stream-url {
  font-size: 11px;
  color: var(--accent-purple);
  background: rgba(180, 77, 255, 0.1);
  padding: 3px 10px;
  border-radius: var(--radius-sm);
  max-width: 300px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.control-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.toggle-switch {
  width: 36px;
  height: 20px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  cursor: pointer;
  position: relative;
  transition: all 0.25s;
}

.toggle-switch::after {
  content: '';
  position: absolute;
  width: 14px;
  height: 14px;
  background: var(--text-muted);
  border-radius: 50%;
  top: 2px;
  left: 2px;
  transition: all 0.25s;
}

.toggle-switch.active {
  background: rgba(180, 77, 255, 0.2);
  border-color: var(--accent-purple);
}

.toggle-switch.active::after {
  background: var(--accent-purple);
  left: 18px;
}

.preview-main {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.preview-container {
  width: 100%;
  height: 100%;
  border-radius: var(--radius-lg);
  overflow: hidden;
  position: relative;
  background: var(--bg-primary);
  border: 1px solid var(--border);
}

.preview-container.control-mode {
  border-color: rgba(180, 77, 255, 0.4);
  box-shadow: 0 0 20px rgba(180, 77, 255, 0.1);
}

.preview-canvas {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-mjpeg {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #0f1523;
}

.preview-placeholder, .preview-starting, .preview-error {
  text-align: center;
  color: var(--text-muted);
}

.preview-placeholder-text, .starting-text, .error-text {
  font-size: 14px;
  font-weight: 600;
  margin-top: 12px;
}

.preview-placeholder-sub, .starting-sub {
  font-size: 11px;
  opacity: 0.5;
  margin-top: 4px;
}

.starting-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(180, 77, 255, 0.2);
  border-top-color: var(--accent-purple);
  border-radius: 50%;
  margin: 0 auto;
  animation: spin 1s linear infinite;
}

.starting-spinner.purple {
  border: 3px solid rgba(180, 77, 255, 0.2);
  border-top-color: var(--accent-purple);
}

.starting-text { color: var(--accent-purple); }
.error-text { color: var(--accent-red); }
.disconnected-text { color: var(--text-muted); font-size: 14px; font-weight: 600; margin-top: 12px; }
.error-detail { font-size: 11px; color: var(--text-muted); max-width: 300px; word-break: break-all; margin-top: 6px; }

@keyframes spin { to { transform: rotate(360deg); } }

.preview-hud {
  position: absolute;
  top: 12px;
  left: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.hud-item {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--accent-purple);
  background: rgba(10, 14, 23, 0.8);
  backdrop-filter: blur(8px);
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid rgba(180, 77, 255, 0.2);
}

.preview-hud-right {
  position: absolute;
  top: 12px;
  right: 12px;
}

.fps-counter {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--accent-green);
  background: rgba(10, 14, 23, 0.8);
  backdrop-filter: blur(8px);
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid rgba(0, 255, 136, 0.2);
}

.control-indicator {
  position: absolute;
  bottom: 12px;
  left: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--accent-purple);
  background: rgba(10, 14, 23, 0.8);
  backdrop-filter: blur(8px);
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid rgba(180, 77, 255, 0.3);
}

.control-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent-purple);
  animation: pulse 1.5s ease infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.font-mono { font-family: var(--font-mono); }
</style>
