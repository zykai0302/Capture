<script setup lang="ts">
import { ref, watch, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { useConfig, usePipeline } from '../composables'
import type { EncodeConfig } from '../types'
import { Codec, EncodeMode, RateControl, EncodePreset, defaultEncodeConfig } from '../types'

const props = defineProps<{
  sourceId: string | null
}>()

const { config, gpuCaps, getAvailableEncoders } = inject<ReturnType<typeof useConfig>>('config')!
const { refreshStatus } = inject<ReturnType<typeof usePipeline>>('pipeline')!

const encodeConfig = ref<EncodeConfig>(defaultEncodeConfig())
const availableEncoders = ref<string[]>([])

watch(() => props.sourceId, () => {
  if (config.value) {
    encodeConfig.value = { ...config.value.default_encode }
  }
})

async function loadEncoders() {
  availableEncoders.value = await getAvailableEncoders()
}
loadEncoders()

async function applyConfig() {
  if (!props.sourceId) return
  try {
    await invoke('update_encode_config', {
      sourceId: props.sourceId,
      config: encodeConfig.value,
    })
    await refreshStatus()
  } catch (e) {
    console.error('Failed to apply config:', e)
  }
}

const resolutionOptions = [
  { label: '原始分辨率', value: 'Original' },
  { label: '1920x1080', value: '1920x1080' },
  { label: '1280x720', value: '1280x720' },
]

const selectedResolution = ref('Original')

watch(selectedResolution, (val) => {
  if (val === 'Original') {
    encodeConfig.value.resolution = { Original: true }
  } else {
    const [w, h] = val.split('x').map(Number)
    encodeConfig.value.resolution = { Custom: { width: w, height: h } }
  }
})

function getGpuLabel(): string {
  if (!gpuCaps.value) return 'Detecting...'
  if (gpuCaps.value.has_amf) return 'AMD GPU (AMF)'
  if (gpuCaps.value.has_mf) return 'GPU (MF)'
  if (gpuCaps.value.has_videotoolbox) return 'Apple GPU (VideoToolbox)'
  if (gpuCaps.value.has_vaapi) return 'GPU (VAAPI)'
  return 'CPU Only'
}
</script>

<template>
  <div class="encoding-config">
    <div class="config-section">
      <div class="config-section-title">编码器</div>
      <div class="config-row">
        <span class="config-label">视频编码</span>
        <div class="config-value">
          <select class="custom-select" v-model="encodeConfig.codec">
            <option :value="Codec.H264">H.264</option>
            <option :value="Codec.H265">H.265 (HEVC)</option>
          </select>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">编码模式</span>
        <div class="config-value">
          <select class="custom-select" v-model="encodeConfig.mode">
            <option :value="EncodeMode.Auto">自动 (GPU优先)</option>
            <option :value="EncodeMode.GpuOnly">仅 GPU 硬编</option>
            <option :value="EncodeMode.CpuOnly">仅 CPU 软编</option>
          </select>
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">画面参数</div>
      <div class="config-row">
        <span class="config-label">分辨率</span>
        <div class="config-value">
          <select class="custom-select" v-model="selectedResolution">
            <option v-for="opt in resolutionOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
          </select>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">帧率</span>
        <div class="config-value">
          <div class="range-container">
            <input type="range" class="custom-range" min="10" max="60" v-model.number="encodeConfig.framerate">
            <span class="input-unit">{{ encodeConfig.framerate }} fps</span>
          </div>
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">码率控制</div>
      <div class="config-row">
        <span class="config-label">码率模式</span>
        <div class="config-value">
          <select class="custom-select" v-model="encodeConfig.rate_control">
            <option :value="RateControl.VBR">VBR</option>
            <option :value="RateControl.CBR">CBR</option>
          </select>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">目标码率</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="encodeConfig.bitrate_kbps" min="500" max="20000">
          <span class="input-unit">kbps</span>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">最大码率</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="encodeConfig.max_bitrate_kbps" min="500" max="30000">
          <span class="input-unit">kbps</span>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">GOP 大小</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="encodeConfig.gop_size" min="10" max="120">
          <span class="input-unit">帧</span>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">编码预设</span>
        <div class="config-value">
          <select class="custom-select" v-model="encodeConfig.preset">
            <option :value="EncodePreset.Balanced">均衡 (balanced)</option>
            <option :value="EncodePreset.Speed">速度优先 (speed)</option>
            <option :value="EncodePreset.Quality">质量优先 (quality)</option>
          </select>
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">GPU 信息</div>
      <div class="config-row">
        <span class="config-label">检测到 GPU</span>
        <div class="config-value">
          <span class="gpu-badge amd">{{ getGpuLabel() }}</span>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">GPU 编码能力</span>
        <div class="config-value">
          <span style="font-family:var(--font-mono);font-size:11px;color:var(--accent-green);">
            {{ gpuCaps?.has_amf ? 'H.264 + H.265 (AMF)' : gpuCaps?.has_mf ? 'H.264 (MF)' : gpuCaps?.has_videotoolbox ? 'H.264 + H.265 (VT)' : gpuCaps?.has_vaapi ? 'H.264 + H.265 (VAAPI)' : '无' }}
          </span>
        </div>
      </div>
    </div>

    <button class="apply-btn" @click="applyConfig" :disabled="!sourceId">
      应用配置
    </button>
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

.custom-select {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 5px 28px 5px 10px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%235a6e8f' stroke-width='1.5' fill='none'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  transition: all 0.2s;
}

.custom-select:hover {
  border-color: var(--border-active);
}

.custom-select:focus {
  outline: none;
  border-color: var(--accent-cyan);
  box-shadow: 0 0 0 2px var(--accent-cyan-glow);
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

.custom-input:hover {
  border-color: var(--border-active);
}

.custom-input:focus {
  outline: none;
  border-color: var(--accent-cyan);
  box-shadow: 0 0 0 2px var(--accent-cyan-glow);
}

.input-unit {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

.range-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.custom-range {
  -webkit-appearance: none;
  appearance: none;
  width: 120px;
  height: 4px;
  background: var(--border);
  border-radius: 2px;
  outline: none;
}

.custom-range::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  background: var(--accent-cyan);
  border-radius: 50%;
  cursor: pointer;
  box-shadow: 0 0 6px var(--accent-cyan-glow);
}

.gpu-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 3px 8px;
  border-radius: 4px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}

.gpu-badge.amd {
  color: #ed1c24;
  border-color: rgba(237, 28, 36, 0.3);
}

.apply-btn {
  width: 100%;
  padding: 8px 0;
  background: var(--accent-cyan);
  color: var(--bg-deep);
  border: none;
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  margin-top: 8px;
}

.apply-btn:hover:not(:disabled) {
  background: var(--accent-cyan-dim);
  box-shadow: 0 0 15px var(--accent-cyan-glow);
}

.apply-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
