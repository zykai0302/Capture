<script setup lang="ts">
import { computed } from 'vue'
import type { CaptureSource, PipelineStatus } from '../types'
import PipelineVisual from './PipelineVisual.vue'

const props = defineProps<{
  selectedSource: CaptureSource | null
  pipelineStatus?: PipelineStatus
}>()

const emit = defineEmits<{
  stopStream: [sourceId: string]
}>()

const isRunning = computed(() =>
  !!props.pipelineStatus && props.pipelineStatus.state === 'Running'
)

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
            <!-- Running state: show placeholder content -->
            <svg viewBox="0 0 960 540" style="width:100%;height:100%;">
              <rect width="960" height="540" fill="#0f1523"/>
              <rect x="0" y="500" width="960" height="40" fill="#151d2e"/>
              <rect x="10" y="508" width="25" height="25" rx="3" fill="#1e2d45"/>
              <rect x="45" y="508" width="25" height="25" rx="3" fill="#1e2d45"/>
              <rect x="80" y="508" width="25" height="25" rx="3" fill="#1e2d45"/>
              <rect x="30" y="30" width="60" height="50" rx="6" fill="#1e2d45"/>
              <rect x="30" y="85" width="60" height="8" rx="2" fill="#5a6e8f"/>
              <rect x="140" y="20" width="500" height="350" rx="8" fill="#1a2438" stroke="#2a4a6b" stroke-width="1"/>
              <rect x="140" y="20" width="500" height="32" rx="8" fill="#253350"/>
              <rect x="148" y="28" width="10" height="10" rx="5" fill="#ff5f57"/>
              <rect x="162" y="28" width="10" height="10" rx="5" fill="#febc2e"/>
              <rect x="176" y="28" width="10" height="10" rx="5" fill="#28c840"/>
              <rect x="160" y="65" width="200" height="10" rx="2" fill="#2a4a6b"/>
              <rect x="160" y="85" width="460" height="6" rx="2" fill="#1e2d45"/>
              <rect x="160" y="100" width="420" height="6" rx="2" fill="#1e2d45"/>
              <rect x="670" y="20" width="260" height="350" rx="8" fill="#1a2438" stroke="#2a4a6b" stroke-width="1"/>
              <rect x="0" y="0" width="960" height="2" fill="rgba(0,212,255,0.06)" rx="1">
                <animate attributeName="y" from="-2" to="540" dur="4s" repeatCount="indefinite"/>
              </rect>
            </svg>
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
