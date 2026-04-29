<script setup lang="ts">
import { ref, watch, inject } from 'vue'
import type { useConfig, usePipeline } from '../composables'

const { config } = inject<ReturnType<typeof useConfig>>('config')!
const { pipelines } = inject<ReturnType<typeof usePipeline>>('pipeline')!

const rtspPort = ref(config.value?.rtsp_port ?? 8554)
const maxClients = ref(config.value?.rtsp_max_clients ?? 10)

const streamingPipelines = ref<{ sourceId: string; rtspUrl: string }[]>([])

// Update streaming URLs from pipeline status reactively
watch(pipelines, () => {
  const urls: { sourceId: string; rtspUrl: string }[] = []
  for (const [sourceId, status] of Object.entries(pipelines.value)) {
    if (status.state === 'Running') {
      urls.push({ sourceId, rtspUrl: status.rtsp_url })
    }
  }
  streamingPipelines.value = urls
}, { immediate: true, deep: true })

function copyUrl(url: string) {
  navigator.clipboard?.writeText(url)
}
</script>

<template>
  <div class="network-config">
    <div class="config-section">
      <div class="config-section-title">RTSP 服务</div>
      <div class="config-row">
        <span class="config-label">RTSP 端口</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="rtspPort">
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">最大客户端数</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="maxClients" min="1" max="50">
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">推流地址</div>
      <div v-for="item in streamingPipelines" :key="item.sourceId" class="rtsp-url-box">
        <div class="rtsp-url-label">{{ item.sourceId }}</div>
        <div class="rtsp-url-value">
          {{ item.rtspUrl }}
          <button class="copy-btn" @click="copyUrl(item.rtspUrl)">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
          </button>
        </div>
      </div>
      <div v-if="streamingPipelines.length === 0" class="empty-hint">
        暂无活跃推流
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">网络状态</div>
      <div class="config-row">
        <span class="config-label">活跃推流</span>
        <div class="config-value">
          <span class="stat-value green">{{ streamingPipelines.length }} 路</span>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">自动重连</span>
        <div class="config-value">
          <div class="toggle-switch active"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-section {
  margin-bottom: 20px;
}

.config-section-title {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  color: var(--text-muted);
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.config-section-title::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--border);
}

.config-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.config-row + .config-row {
  border-top: 1px solid rgba(30, 45, 69, 0.5);
}

.config-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.config-value {
  display: flex;
  align-items: center;
  gap: 6px;
}

.custom-input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 5px 10px;
  width: 90px;
  text-align: right;
  transition: all 0.2s;
}

.custom-input:focus {
  outline: none;
  border-color: var(--accent-cyan);
  box-shadow: 0 0 0 2px var(--accent-cyan-glow);
}

.rtsp-url-box {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px 12px;
  margin-top: 8px;
}

.rtsp-url-label {
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.rtsp-url-value {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--accent-cyan);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.copy-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px;
  transition: color 0.2s;
}

.copy-btn:hover {
  color: var(--accent-cyan);
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
  background: var(--accent-cyan-glow);
  border-color: var(--accent-cyan);
}

.toggle-switch.active::after {
  background: var(--accent-cyan);
  left: 18px;
}

.stat-value {
  font-family: var(--font-mono);
  font-size: 11px;
}

.stat-value.green {
  color: var(--accent-green);
}

.empty-hint {
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
  padding: 20px 0;
}
</style>
