<script setup lang="ts">
import { ref, computed, inject } from 'vue'
import type { useSources, usePipeline } from '../composables'
import SourceItem from './SourceItem.vue'
import type { CaptureSource } from '../types'

defineProps<{
  selectedSourceId: string | null
}>()

const { monitors, windows, allSources } = inject<ReturnType<typeof useSources>>('sources')!
const { pipelines, startStream, stopStream } = inject<ReturnType<typeof usePipeline>>('pipeline')!

type TabType = 'all' | 'monitors' | 'windows'
const activeTab = ref<TabType>('all')

const emit = defineEmits<{
  selectSource: [source: CaptureSource]
}>()

const filteredSources = computed(() => {
  switch (activeTab.value) {
    case 'monitors': return monitors.value
    case 'windows': return windows.value
    default: return allSources.value
  }
})

const availableCount = computed(() => allSources.value.length)

function selectSource(sourceId: string) {
  const source = allSources.value.find(s => s.id === sourceId)
  if (source) emit('selectSource', source)
}

async function onStartStream(source: CaptureSource) {
  // Auto-select the source when starting stream so PipelineVisual shows its status
  emit('selectSource', source)
  try {
    await startStream(source)
  } catch (e) {
    console.error('Failed to start stream:', e)
  }
}

async function onStopStream(sourceId: string) {
  try {
    await stopStream(sourceId)
  } catch (e) {
    console.error('Failed to stop stream:', e)
  }
}
</script>

<template>
  <aside class="panel-left">
    <div class="panel-header">
      <span class="panel-title">画面源</span>
      <span class="panel-count">{{ availableCount }} 可用</span>
    </div>

    <div class="source-tabs">
      <button class="source-tab" :class="{ active: activeTab === 'all' }" @click="activeTab = 'all'">全部</button>
      <button class="source-tab" :class="{ active: activeTab === 'monitors' }" @click="activeTab = 'monitors'">显示器</button>
      <button class="source-tab" :class="{ active: activeTab === 'windows' }" @click="activeTab = 'windows'">窗口</button>
    </div>

    <div class="source-list">
      <SourceItem
        v-for="source in filteredSources"
        :key="source.id"
        :source="source"
        :pipeline-status="pipelines[source.id]"
        :selected="selectedSourceId === source.id"
        @select="selectSource"
        @start-stream="onStartStream"
        @stop-stream="onStopStream"
      />
      <div v-if="filteredSources.length === 0" class="empty-hint">
        未检测到画面源
      </div>
    </div>
  </aside>
</template>

<style scoped>
.panel-left {
  grid-area: left;
  background: var(--bg-primary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  padding: 16px 16px 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.panel-title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  color: var(--text-secondary);
}

.panel-count {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--accent-cyan);
  background: var(--accent-cyan-glow);
  padding: 2px 8px;
  border-radius: 10px;
}

.source-tabs {
  display: flex;
  padding: 0 16px;
  gap: 2px;
  flex-shrink: 0;
}

.source-tab {
  flex: 1;
  padding: 8px 0;
  font-size: 11px;
  font-weight: 500;
  text-align: center;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.source-tab.active {
  color: var(--accent-cyan);
  border-bottom-color: var(--accent-cyan);
}

.source-tab:hover:not(.active) {
  color: var(--text-secondary);
}

.source-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.empty-hint {
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
  padding: 40px 0;
}
</style>
