<script setup lang="ts">
import { computed } from 'vue'
import type { CaptureSource, PipelineStatus } from '../types'
import { SourceType, getPipelineStateLabel, isPipelineError, isPipelineRunning } from '../types'

const props = defineProps<{
  source: CaptureSource
  pipelineStatus?: PipelineStatus
  selected?: boolean
}>()

const emit = defineEmits<{
  select: [sourceId: string]
  startStream: [source: CaptureSource]
  stopStream: [sourceId: string]
}>()

const isStreaming = computed(() =>
  props.pipelineStatus ? isPipelineRunning(props.pipelineStatus.state) : props.source.is_streaming
)

const statusText = computed(() => {
  if (props.pipelineStatus) {
    return getPipelineStateLabel(props.pipelineStatus.state)
  }
  return props.source.is_streaming ? '推流中' : '就绪'
})

const isError = computed(() =>
  props.pipelineStatus ? isPipelineError(props.pipelineStatus.state) : false
)

const rtspUrl = computed(() =>
  props.pipelineStatus?.rtsp_url || ''
)

function copyUrl() {
  if (rtspUrl.value) {
    navigator.clipboard?.writeText(rtspUrl.value)
  }
}
</script>

<template>
  <div
    class="source-item"
    :class="{
      streaming: isStreaming,
      selected: selected,
      error: isError,
    }"
    @click="emit('select', source.id)"
  >
    <div class="source-item-top">
      <div class="source-preview">
        <svg class="screen-preview-svg" viewBox="0 0 160 90">
          <rect width="160" height="90" fill="#1a2438"/>
          <template v-if="source.source_type === SourceType.Monitor">
            <rect x="10" y="8" width="60" height="35" rx="3" fill="#2a4a6b" opacity="0.5"/>
            <rect x="80" y="8" width="70" height="20" rx="3" fill="#1e2d45" opacity="0.5"/>
            <rect x="10" y="50" width="140" height="8" rx="2" fill="#1e2d45"/>
            <rect x="10" y="64" width="90" height="8" rx="2" fill="#1e2d45"/>
            <rect x="10" y="78" width="50" height="8" rx="2" fill="#1e2d45"/>
          </template>
          <template v-else>
            <rect x="15" y="5" width="130" height="80" rx="4" fill="#1e2d45" stroke="#2a4a6b" stroke-width="1"/>
            <rect x="15" y="5" width="130" height="16" rx="4" fill="#253350"/>
            <circle cx="27" cy="13" r="3" fill="#ff5f57"/>
            <circle cx="37" cy="13" r="3" fill="#febc2e"/>
            <circle cx="47" cy="13" r="3" fill="#28c840"/>
            <rect x="25" y="28" width="110" height="6" rx="2" fill="#2a4a6b"/>
            <rect x="25" y="40" width="80" height="4" rx="2" fill="#253350"/>
          </template>
        </svg>
        <span v-if="isStreaming" class="live-badge">LIVE</span>
      </div>
      <div class="source-info">
        <div class="source-name">{{ source.name }}</div>
        <div class="source-meta">
          <span class="source-type-badge" :class="source.source_type === SourceType.Monitor ? 'badge-monitor' : 'badge-window'">
            <svg v-if="source.source_type === SourceType.Monitor" width="8" height="8" viewBox="0 0 24 24" fill="currentColor">
              <rect x="2" y="3" width="20" height="14" rx="2"/><rect x="7" y="21" width="10" height="2" rx="1"/>
            </svg>
            <svg v-else width="8" height="8" viewBox="0 0 24 24" fill="currentColor">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
            </svg>
            {{ source.source_type === SourceType.Monitor ? '显示器' : '窗口' }}
          </span>
          <span>{{ source.width }}x{{ source.height }}</span>
        </div>
      </div>
    </div>
    <div class="source-item-bottom">
      <div class="source-status" :class="{ 'status-streaming': isStreaming, 'status-idle': !isStreaming && !isError, 'status-error': isError }">
        <span class="status-indicator"></span>
        <span class="status-text">{{ statusText }}</span>
        <span v-if="isStreaming && pipelineStatus?.encoder_used" class="codec-badge" :class="pipelineStatus.is_gpu ? 'codec-hw' : 'codec-sw'">
          {{ pipelineStatus.encoder_used }}
        </span>
      </div>
      <div class="source-actions">
        <button
          v-if="!isStreaming"
          class="source-action-btn btn-start"
          title="开始推流"
          @click.stop="emit('startStream', source)"
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
        </button>
        <button
          v-if="isStreaming"
          class="source-action-btn btn-stop"
          title="停止推流"
          @click.stop="emit('stopStream', source.id)"
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
        </button>
      </div>
    </div>
    <div v-if="isStreaming" class="encoding-bar">
      <div class="encoding-bar-fill" :class="pipelineStatus?.is_gpu ? 'gpu' : 'cpu'" :style="{ width: '35%' }"></div>
    </div>
    <div v-if="isStreaming && rtspUrl" class="rtsp-url-row" @click.stop="copyUrl">
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
      </svg>
      <span class="rtsp-url-text">{{ rtspUrl }}</span>
    </div>
  </div>
</template>

<style scoped>
.source-item {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px;
  cursor: pointer;
  transition: all 0.25s;
  position: relative;
  overflow: hidden;
  animation: slideInUp 0.3s ease forwards;
}

.source-item::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: transparent;
  transition: background 0.25s;
}

.source-item:hover {
  border-color: var(--border-active);
  background: var(--bg-hover);
}

.source-item.streaming {
  border-color: var(--accent-green);
  background: rgba(0, 255, 136, 0.03);
}

.source-item.streaming::before {
  background: var(--accent-green);
}

.source-item.selected {
  border-color: var(--accent-cyan);
  background: var(--accent-cyan-glow);
}

.source-item.selected::before {
  background: var(--accent-cyan);
}

.source-item.error {
  border-color: var(--accent-red);
}

.source-item-top {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-bottom: 8px;
}

.source-preview {
  width: 80px;
  height: 45px;
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  overflow: hidden;
  position: relative;
  flex-shrink: 0;
}

.screen-preview-svg {
  width: 100%;
  height: 100%;
}

.live-badge {
  position: absolute;
  top: 3px;
  left: 3px;
  font-family: var(--font-mono);
  font-size: 8px;
  font-weight: 700;
  color: var(--accent-red);
  background: rgba(0,0,0,0.7);
  padding: 1px 4px;
  border-radius: 3px;
  animation: blink-live 1.5s infinite;
}

.source-info {
  flex: 1;
  min-width: 0;
}

.source-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 3px;
}

.source-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

.source-type-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
}

.badge-monitor {
  background: var(--accent-cyan-glow);
  color: var(--accent-cyan);
}

.badge-window {
  background: rgba(180, 77, 255, 0.15);
  color: #d4a0ff;
}

.source-item-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.source-status {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
}

.status-indicator {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.status-idle .status-indicator { background: var(--text-muted); }
.status-idle .status-text { color: var(--text-muted); }
.status-streaming .status-indicator { background: var(--accent-green); box-shadow: 0 0 4px var(--accent-green); }
.status-streaming .status-text { color: var(--accent-green); }
.status-error .status-indicator { background: var(--accent-red); box-shadow: 0 0 4px var(--accent-red); }
.status-error .status-text { color: var(--accent-red); }

.codec-badge {
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
}

.codec-hw {
  background: var(--accent-green-glow);
  color: var(--accent-green);
  border: 1px solid rgba(0, 255, 136, 0.3);
}

.codec-sw {
  background: var(--accent-orange-glow);
  color: var(--accent-orange);
  border: 1px solid rgba(255, 140, 0, 0.3);
}

.source-actions {
  display: flex;
  gap: 4px;
}

.source-action-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.2s;
  font-size: 12px;
}

.source-action-btn:hover {
  background: var(--bg-hover);
  color: var(--accent-cyan);
  border-color: var(--border-active);
}

.source-action-btn.btn-start:hover {
  color: var(--accent-green);
  border-color: var(--accent-green);
}

.source-action-btn.btn-stop:hover {
  color: var(--accent-red);
  border-color: var(--accent-red);
}

.encoding-bar {
  height: 3px;
  background: var(--bg-secondary);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 6px;
}

.encoding-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s, background 0.3s;
}

.encoding-bar-fill.gpu {
  background: linear-gradient(90deg, var(--accent-cyan), var(--accent-green));
}

.encoding-bar-fill.cpu {
  background: linear-gradient(90deg, var(--accent-orange), var(--accent-red));
}

.rtsp-url-row {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 6px;
  padding: 3px 8px;
  border-radius: var(--radius-sm);
  background: var(--accent-cyan-glow);
  color: var(--accent-cyan);
  font-family: var(--font-mono);
  font-size: 9px;
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rtsp-url-row:hover {
  background: rgba(0, 212, 255, 0.25);
}

.rtsp-url-text {
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
