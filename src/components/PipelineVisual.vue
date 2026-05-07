<script setup lang="ts">
import { computed } from 'vue'
import type { PipelineStatus } from '../types'
import { isPipelineError, isPipelineRunning } from '../types'

const props = defineProps<{
  pipelineStatus?: PipelineStatus
}>()

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

// Per-node state: capture → encode → mux → send
// Each node can be: active, starting, error, inactive
function nodeState(phase: 'capture' | 'encode' | 'mux' | 'send'): string {
  if (isError.value) return 'error'
  if (isStarting.value) {
    // Starting means capture is starting, later phases not yet
    if (phase === 'capture') return 'starting'
    return 'inactive'
  }
  if (isRunning.value) return 'active'
  return 'inactive'
}
</script>

<template>
  <div class="pipeline-visual">
    <div class="pipeline-label">Pipeline 状态</div>
    <!-- Error banner -->
    <div v-if="errorMsg" class="pipeline-error-banner">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
      <span>{{ errorMsg }}</span>
    </div>
    <div class="pipeline-flow">
      <!-- Capture Node -->
      <div class="pipeline-node" :class="[nodeState('capture'), 'node-capture']">
        <span class="pipeline-node-icon">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
            <rect x="2" y="3" width="20" height="14" rx="2"/><rect x="7" y="21" width="10" height="2" rx="1"/>
          </svg>
        </span>
        <span>
          <div class="pipeline-node-label">屏幕捕获</div>
          <div class="pipeline-node-detail">d3d12screencapturesrc</div>
        </span>
      </div>

      <!-- Arrow 1 -->
      <div class="pipeline-arrow" :class="{ active: isRunning, starting: isStarting, error: isError }">
        <svg width="24" height="8" viewBox="0 0 24 8"><path d="M0 4h20m0 0l-4-3.5M20 4l-4 3.5" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
      </div>

      <!-- Encode Node -->
      <div class="pipeline-node" :class="[nodeState('encode'), 'node-encode']">
        <span class="pipeline-node-icon">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10"/>
          </svg>
        </span>
        <span>
          <div class="pipeline-node-label">{{ pipelineStatus?.encoder_used || 'H.264' }}</div>
          <div class="pipeline-node-detail">{{ pipelineStatus?.bitrate_kbps || 4000 }}kbps {{ pipelineStatus?.is_gpu ? 'GPU' : 'CPU' }}</div>
        </span>
      </div>

      <!-- Arrow 2 -->
      <div class="pipeline-arrow" :class="{ active: isRunning, starting: isStarting, error: isError }">
        <svg width="24" height="8" viewBox="0 0 24 8"><path d="M0 4h20m0 0l-4-3.5M20 4l-4 3.5" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
      </div>

      <!-- Mux Node -->
      <div class="pipeline-node" :class="[nodeState('mux'), 'node-mux']">
        <span class="pipeline-node-icon">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>
          </svg>
        </span>
        <span>
          <div class="pipeline-node-label">RTP 封装</div>
          <div class="pipeline-node-detail">rtph264pay</div>
        </span>
      </div>

      <!-- Arrow 3 -->
      <div class="pipeline-arrow" :class="{ active: isRunning, starting: isStarting, error: isError }">
        <svg width="24" height="8" viewBox="0 0 24 8"><path d="M0 4h20m0 0l-4-3.5M20 4l-4 3.5" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
      </div>

      <!-- Send Node -->
      <div class="pipeline-node" :class="[nodeState('send'), 'node-send']">
        <span class="pipeline-node-icon">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
          </svg>
        </span>
        <span>
          <div class="pipeline-node-label">RTSP 推流</div>
          <div class="pipeline-node-detail">{{ pipelineStatus?.rtsp_url || 'rtsp://127.0.0.1:8554' }}</div>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pipeline-visual {
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  overflow-x: auto;
}

.pipeline-label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1.2px;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.pipeline-flow {
  display: flex;
  align-items: center;
  gap: 0;
  min-width: max-content;
}

.pipeline-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-weight: 500;
  white-space: nowrap;
  transition: all 0.3s;
}

.pipeline-node.active {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-active);
}

.pipeline-node.inactive {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  opacity: 0.5;
}

.pipeline-node.starting {
  background: rgba(0, 212, 255, 0.08);
  border: 1px solid var(--accent-cyan);
  animation: pulse-border 1.5s ease infinite;
}

.pipeline-node.error {
  background: rgba(255, 80, 80, 0.08);
  border: 1px solid var(--accent-red);
  opacity: 0.85;
}

.pipeline-node-icon {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-size: 10px;
}

.node-capture .pipeline-node-icon { background: var(--accent-cyan-glow); color: var(--accent-cyan); }
.node-encode .pipeline-node-icon { background: var(--accent-green-glow); color: var(--accent-green); }
.node-mux .pipeline-node-icon { background: var(--accent-orange-glow); color: var(--accent-orange); }
.node-send .pipeline-node-icon { background: rgba(180, 77, 255, 0.15); color: var(--accent-purple); }

.pipeline-node-label {
  color: var(--text-primary);
}

.pipeline-node-detail {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
}

.pipeline-arrow {
  display: flex;
  align-items: center;
  padding: 0 4px;
  color: var(--border-active);
  position: relative;
}

.pipeline-arrow.active {
  color: var(--accent-green);
}

.pipeline-arrow.starting {
  color: var(--accent-cyan);
  opacity: 0.6;
}

.pipeline-arrow.error {
  color: var(--accent-red);
  opacity: 0.6;
}

.pipeline-arrow.active::after {
  content: '';
  position: absolute;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--accent-green);
  box-shadow: 0 0 6px var(--accent-green);
  animation: flow-particle 1.5s infinite;
}

.pipeline-error-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin-bottom: 10px;
  background: rgba(255, 80, 80, 0.1);
  border: 1px solid rgba(255, 80, 80, 0.3);
  border-radius: var(--radius-sm);
  color: var(--accent-red);
  font-size: 11px;
  word-break: break-all;
}

@keyframes pulse-border {
  0%, 100% { border-color: var(--accent-cyan); opacity: 1; }
  50% { border-color: rgba(0, 212, 255, 0.4); opacity: 0.7; }
}
</style>
