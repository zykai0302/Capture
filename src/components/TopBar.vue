<script setup lang="ts">
import { computed, inject } from 'vue'
import type { useConfig, usePipeline } from '../composables'

const { config, gpuCaps } = inject<ReturnType<typeof useConfig>>('config')!
const { pipelines } = inject<ReturnType<typeof usePipeline>>('pipeline')!

const emit = defineEmits<{
  toggleSettings: []
}>()

defineProps<{
  settingsVisible: boolean
}>()

const streamingCount = computed(() => {
  let count = 0
  for (const p of Object.values(pipelines.value)) {
    if (p.state === 'Running') count++
  }
  return count
})

const gpuInfo = computed(() => {
  if (!gpuCaps.value) return 'Detecting...'
  if (gpuCaps.value.has_amf) return 'AMD GPU (AMF)'
  if (gpuCaps.value.has_mf) return 'GPU (MF)'
  if (gpuCaps.value.has_videotoolbox) return 'Apple GPU (VideoToolbox)'
  if (gpuCaps.value.has_vaapi) return 'GPU (VAAPI)'
  return 'CPU Only'
})
</script>

<template>
  <header class="topbar">
    <div class="topbar-left">
      <div class="logo">
        <div class="logo-icon">
          <svg viewBox="0 0 28 28" fill="none">
            <rect x="2" y="2" width="24" height="24" rx="4" stroke="url(#logo-grad)" stroke-width="2"/>
            <rect x="6" y="6" width="16" height="12" rx="2" fill="url(#logo-grad)" opacity="0.3"/>
            <circle cx="14" cy="12" r="3" fill="url(#logo-grad)"/>
            <path d="M10 22h8" stroke="url(#logo-grad)" stroke-width="2" stroke-linecap="round"/>
            <defs>
              <linearGradient id="logo-grad" x1="2" y1="2" x2="26" y2="26">
                <stop stop-color="#00d4ff"/>
                <stop offset="1" stop-color="#00ff88"/>
              </linearGradient>
            </defs>
          </svg>
        </div>
        <span class="logo-text">SCREENCAST PRO</span>
        <span class="logo-version">v0.1.0</span>
      </div>
    </div>
    <div class="topbar-center">
      <div class="system-stats">
        <div class="stat-item">
          <span class="stat-dot green"></span>
          <span>GPU: {{ gpuInfo }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-dot cyan"></span>
          <span>RTSP :{{ config?.rtsp_port ?? 8554 }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-dot orange"></span>
          <span>推流: {{ streamingCount }} 路</span>
        </div>
      </div>
    </div>
    <div class="topbar-right">
      <button class="topbar-btn btn-settings" :class="{ active: settingsVisible }" @click="emit('toggleSettings')">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
          <circle cx="12" cy="12" r="3"/>
        </svg>
        设置
      </button>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  grid-area: topbar;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border);
  position: relative;
  z-index: 10;
}

.topbar::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--accent-cyan), transparent);
  opacity: 0.3;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo-icon {
  width: 28px;
  height: 28px;
}

.logo-icon svg {
  width: 100%;
  height: 100%;
}

.logo-text {
  font-weight: 700;
  font-size: 16px;
  letter-spacing: 1px;
  background: linear-gradient(135deg, var(--accent-cyan), var(--accent-green));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.logo-version {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
}

.topbar-center {
  display: flex;
  align-items: center;
  gap: 24px;
}

.system-stats {
  display: flex;
  align-items: center;
  gap: 20px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-secondary);
}

.stat-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  animation: pulse-dot 2s infinite;
}

.stat-dot.green { background: var(--accent-green); box-shadow: 0 0 6px var(--accent-green); }
.stat-dot.cyan { background: var(--accent-cyan); box-shadow: 0 0 6px var(--accent-cyan); }
.stat-dot.orange { background: var(--accent-orange); box-shadow: 0 0 6px var(--accent-orange); }

.topbar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.topbar-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.topbar-btn:hover {
  background: var(--bg-hover);
  border-color: var(--border-active);
  color: var(--accent-cyan);
}

.btn-settings {
  width: auto;
  padding: 0 12px;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
}

.btn-settings.active {
  background: rgba(0, 200, 255, 0.15);
  border-color: #00c8ff;
  color: #00c8ff;
}
</style>
