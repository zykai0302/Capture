<script setup lang="ts">
import { ref, watch, computed, inject, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CaptureSource, PipelineStatus } from '../types'
import { isPipelineError, isPipelineRunning } from '../types'
import type { useConfig } from '../composables'
import PipelineVisual from './PipelineVisual.vue'

const props = defineProps<{
  selectedSource: CaptureSource | null
  pipelineStatus?: PipelineStatus
}>()

const emit = defineEmits<{
  stopStream: [sourceId: string]
}>()

// Inject config store for preview port
const { config } = inject<ReturnType<typeof useConfig>>('config')!

// Preview URL for MJPEG stream
const previewUrl = ref<string>('')
// Key to force <img> re-creation to establish fresh MJPEG connection
const imgKey = ref(0)

// Computed preview port from config, fallback to 8090
const previewPort = computed(() => config.value?.preview_http_port ?? 8090)

const isRunning = computed(() =>
  !!props.pipelineStatus && isPipelineRunning(props.pipelineStatus.state)
)

const isStarting = computed(() =>
  !!props.pipelineStatus && props.pipelineStatus.state === 'Starting'
)

const isError = computed(() =>
  !!props.pipelineStatus && isPipelineError(props.pipelineStatus.state)
)

const errorMsg = computed(() => {
  if (isError.value && props.pipelineStatus) {
    const state = props.pipelineStatus.state
    return typeof state === 'object' && 'Error' in state ? state.Error : ''
  }
  return ''
})

// Start preview for the given source
async function startPreview() {
  if (!props.selectedSource) return
  try {
    await invoke('start_preview', { sourceId: props.selectedSource.id })
    previewUrl.value = `http://127.0.0.1:${previewPort.value}/${props.selectedSource.id}`
    imgKey.value++ // Force <img> re-creation to establish fresh MJPEG connection
  } catch (e) {
    console.error('Failed to start preview:', e)
    previewUrl.value = ''
  }
}

// Stop preview and clear the URL
async function stopPreview() {
  previewUrl.value = ''
  if (!props.selectedSource) return
  try {
    await invoke('stop_preview', { sourceId: props.selectedSource.id })
  } catch (e) {
    // Ignore errors — source may already be stopped
  }
}

// Watch streaming state to start/stop preview
watch(isRunning, async (running) => {
  if (running && props.selectedSource) {
    await startPreview()
  } else {
    await stopPreview()
  }
}, { immediate: false })

// On mount: if pipeline is already running, the preview pipeline should
// already be active in the backend (we never stop it on unmount).
// Just set the URL and increment imgKey to establish a fresh MJPEG connection.
onMounted(() => {
  if (isRunning.value && props.selectedSource) {
    previewUrl.value = `http://127.0.0.1:${previewPort.value}/${props.selectedSource.id}`
    imgKey.value++
  }
})

function copyUrl(url: string) {
  navigator.clipboard?.writeText(url)
}
</script>

<template>
  <main class="panel-center">
    <div class="center-header">
      <div class="center-title-area">
        <span class="center-title">{{ selectedSource?.name || '未选择画面源' }}</span>
        <span
          v-if="isRunning && pipelineStatus?.rtsp_url"
          class="stream-url"
          @click="copyUrl(pipelineStatus.rtsp_url)"
          title="点击复制"
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
          </svg>
          {{ pipelineStatus.rtsp_url }}
          <svg class="copy-icon" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        </span>
      </div>
      <div v-if="selectedSource" class="center-actions">
        <button
          v-if="isRunning"
          class="center-btn btn-danger"
          @click="emit('stopStream', selectedSource.id)"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
          停止推流
        </button>
      </div>
    </div>

    <PipelineVisual :pipeline-status="pipelineStatus" />

    <div class="main-preview">
      <div class="preview-container">
        <div class="preview-canvas">
          <template v-if="isRunning">
            <img
              v-if="previewUrl"
              :key="imgKey"
              :src="previewUrl"
              class="preview-mjpeg"
              alt="Live preview"
            />
            <div v-else class="preview-loading">加载预览...</div>
          </template>
          <template v-else-if="isStarting">
            <div class="preview-starting">
              <div class="starting-spinner"></div>
              <div class="starting-text">启动推流中...</div>
              <div class="starting-sub">正在初始化捕获与编码管线</div>
            </div>
          </template>
          <template v-else-if="isError">
            <div class="preview-error">
              <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.6">
                <circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
              </svg>
              <div class="error-text">推流失败</div>
              <div class="error-detail">{{ errorMsg }}</div>
            </div>
          </template>
          <template v-else>
            <div class="preview-placeholder">
              <div class="preview-placeholder-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
                  <rect x="2" y="3" width="20" height="14" rx="2"/><rect x="7" y="21" width="10" height="2" rx="1"/>
                </svg>
              </div>
              <div class="preview-placeholder-text">
                {{ selectedSource ? '点击开始推流' : '选择一个画面源' }}
              </div>
              <div class="preview-placeholder-sub">
                {{ selectedSource ? '从左侧列表选择画面源，点击播放按钮开始推流' : '从左侧列表选择一个画面源' }}
              </div>
            </div>
          </template>
        </div>

        <!-- HUD Overlay -->
        <template v-if="isRunning && pipelineStatus">
          <div class="preview-hud">
            <div class="hud-item">{{ selectedSource?.width }}x{{ selectedSource?.height }} {{ pipelineStatus.encoder_used }}</div>
            <div class="hud-item">延迟: {{ pipelineStatus.latency_ms }}ms</div>
          </div>
          <div class="preview-hud-right">
            <div class="fps-counter">{{ pipelineStatus.fps.toFixed(0) }} FPS</div>
          </div>
        </template>
      </div>
    </div>
  </main>
</template>

<style scoped>
.panel-center {
  grid-area: center;
  background: var(--bg-deep);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.center-header {
  padding: 14px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.center-title-area {
  display: flex;
  align-items: center;
  gap: 10px;
}

.center-title {
  font-size: 14px;
  font-weight: 600;
}

.stream-url {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--accent-cyan);
  background: var(--accent-cyan-glow);
  padding: 3px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
}

.stream-url:hover {
  background: rgba(0, 212, 255, 0.25);
}

.stream-url .copy-icon {
  opacity: 0.5;
  transition: opacity 0.2s;
}

.stream-url:hover .copy-icon {
  opacity: 1;
}

.center-actions {
  display: flex;
  gap: 8px;
}

.center-btn {
  padding: 6px 14px;
  font-size: 12px;
  font-weight: 500;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn-danger {
  background: transparent;
  color: var(--accent-red);
  border-color: var(--accent-red);
}

.btn-danger:hover {
  background: var(--accent-red-glow);
}

.main-preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  padding: 20px;
}

.preview-container {
  width: 100%;
  height: 100%;
  max-width: 100%;
  border-radius: var(--radius-lg);
  overflow: hidden;
  position: relative;
  background: var(--bg-primary);
  border: 1px solid var(--border);
}

.preview-canvas {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-placeholder {
  text-align: center;
  color: var(--text-muted);
}

.preview-placeholder-icon {
  margin-bottom: 16px;
}

.preview-placeholder-text {
  font-size: 14px;
  margin-bottom: 8px;
}

.preview-placeholder-sub {
  font-size: 12px;
  opacity: 0.5;
}

.preview-mjpeg {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #0f1523;
}

.preview-loading {
  text-align: center;
  color: var(--text-muted, #666);
  font-size: 13px;
}

.preview-starting {
  text-align: center;
  color: var(--accent-cyan);
}

.starting-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(0, 212, 255, 0.2);
  border-top-color: var(--accent-cyan);
  border-radius: 50%;
  margin: 0 auto 12px;
  animation: spin 1s linear infinite;
}

.starting-text {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}

.starting-sub {
  font-size: 11px;
  color: var(--text-muted);
}

.preview-error {
  text-align: center;
  color: var(--accent-red);
}

.error-text {
  font-size: 14px;
  font-weight: 600;
  margin-top: 8px;
  margin-bottom: 6px;
}

.error-detail {
  font-size: 11px;
  color: var(--text-muted);
  max-width: 300px;
  word-break: break-all;
  line-height: 1.4;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

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
  color: var(--accent-cyan);
  background: rgba(10, 14, 23, 0.8);
  backdrop-filter: blur(8px);
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid rgba(0, 212, 255, 0.2);
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
</style>
