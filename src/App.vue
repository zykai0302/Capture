<script setup lang="ts">
import { ref, provide } from 'vue'
import TopBar from './components/TopBar.vue'
import SourceList from './components/SourceList.vue'
import MainPreview from './components/MainPreview.vue'
import ConfigPanel from './components/ConfigPanel.vue'
import StatusBar from './components/StatusBar.vue'
import { usePipeline } from './composables/usePipeline'
import { useSources } from './composables/useSources'
import { useConfig } from './composables/useConfig'
import { useRemoteControl } from './composables/useRemoteControl'
import type { CaptureSource } from './types'

// Create shared composable instances (singletons for the app)
const pipelineStore = usePipeline()
const sourcesStore = useSources()
const configStore = useConfig()
const remoteStore = useRemoteControl()

// Provide to all child components
provide('pipeline', pipelineStore)
provide('sources', sourcesStore)
provide('config', configStore)
provide('remote', remoteStore)

const selectedSource = ref<CaptureSource | null>(null)

const showConfigPanel = ref(false)

function onToggleSettings() {
  showConfigPanel.value = !showConfigPanel.value
}

function onSelectSource(source: CaptureSource) {
  selectedSource.value = source
}

async function onStopStream(sourceId: string) {
  try {
    await pipelineStore.stopStream(sourceId)
  } catch (e) {
    console.error('Failed to stop stream:', e)
  }
}
</script>

<template>
  <div class="app" :class="{ 'config-visible': showConfigPanel }" :style="{ gridTemplateColumns: showConfigPanel ? '320px 1fr 340px' : '320px 1fr 0px' }">
    <TopBar :settings-visible="showConfigPanel" @toggle-settings="onToggleSettings" />
    <SourceList
      :selected-source-id="selectedSource?.id ?? null"
      @select-source="onSelectSource"
    />
    <MainPreview
      :selected-source="selectedSource"
      :pipeline-status="selectedSource ? pipelineStore.pipelines.value[selectedSource.id] : undefined"
      @stop-stream="onStopStream"
    />
    <ConfigPanel :selected-source-id="selectedSource?.id ?? null" :visible="showConfigPanel" />
    <StatusBar />
  </div>
</template>

<style>
@import './styles/global.css';
</style>

<style scoped>
.app {
  display: grid;
  grid-template-rows: 52px 1fr 28px;
  grid-template-areas:
    "topbar topbar topbar"
    "left   center right"
    "status status status";
  height: 100vh;
  overflow: hidden;
  transition: grid-template-columns 0.3s ease;
}

@media (max-width: 1200px) {
  .app {
    grid-template-columns: 260px 1fr 0px;
  }
  .app.config-visible {
    grid-template-columns: 260px 1fr 280px;
  }
}
</style>
