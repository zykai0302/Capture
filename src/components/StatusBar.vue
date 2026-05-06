<script setup lang="ts">
import { computed, inject } from 'vue'
import type { usePipeline, useConfig } from '../composables'

const { pipelines } = inject<ReturnType<typeof usePipeline>>('pipeline')!
const { gpuCaps } = inject<ReturnType<typeof useConfig>>('config')!

const streamingCount = computed(() => {
  let count = 0
  for (const p of Object.values(pipelines.value)) {
    if (p.state === 'Running') count++
  }
  return count
})

const gpuLabel = computed(() => {
  if (!gpuCaps.value) return ''
  if (gpuCaps.value.has_amf) return 'AMD GPU'
  if (gpuCaps.value.has_mf) return 'GPU (MF)'
  if (gpuCaps.value.has_videotoolbox) return 'Apple GPU'
  if (gpuCaps.value.has_vaapi) return 'GPU (VAAPI)'
  return 'CPU Only'
})
</script>

<template>
  <div class="bottom-bar">
    <div class="bottom-left">
      <div class="bottom-item">
        <span style="color:var(--accent-green);">&#9679;</span>
        服务运行中
      </div>
      <div class="bottom-item">推流: {{ streamingCount }}路</div>
      <div class="bottom-item">GPU: {{ gpuLabel }}</div>
    </div>
    <div class="bottom-right">
      <div class="bottom-item">GStreamer 1.28</div>
      <div class="bottom-item">Windows</div>
    </div>
  </div>
</template>

<style scoped>
.bottom-bar {
  grid-area: status;
  height: 28px;
  background: var(--bg-primary);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
  z-index: 5;
}

.bottom-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.bottom-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.bottom-right {
  display: flex;
  align-items: center;
  gap: 12px;
}
</style>
