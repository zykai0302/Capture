<script setup lang="ts">
import { ref, provide, watch } from 'vue'
import TopBar from './components/TopBar.vue'
import SourceList from './components/SourceList.vue'
import MainPreview from './components/MainPreview.vue'
import ConfigPanel from './components/ConfigPanel.vue'
import StatusBar from './components/StatusBar.vue'
import RtspStreamList from './components/RtspClient/RtspStreamList.vue'
import RtspPreview from './components/RtspClient/RtspPreview.vue'
import RemoteControlClient from './components/RtspClient/RemoteControlClient.vue'
import { usePipeline } from './composables/usePipeline'
import { useSources } from './composables/useSources'
import { useConfig } from './composables/useConfig'
import { useRemoteControl } from './composables/useRemoteControl'
import { useRtspClient } from './composables/useRtspClient'
import { useWsRemote } from './composables/useWsRemote'
import type { CaptureSource } from './types'

// Create shared composable instances (singletons for the app)
const pipelineStore = usePipeline()
const sourcesStore = useSources()
const configStore = useConfig()
const remoteStore = useRemoteControl()
const rtspClientStore = useRtspClient()
const wsRemoteStore = useWsRemote()

// Provide to all child components
provide('pipeline', pipelineStore)
provide('sources', sourcesStore)
provide('config', configStore)
provide('remote', remoteStore)
provide('rtspClient', rtspClientStore)
provide('wsRemote', wsRemoteStore)

// Mode state
const appMode = ref<'server' | 'client'>('server')
const selectedSource = ref<CaptureSource | null>(null)
const selectedStreamId = ref<string | null>(null)
const serverModeKey = ref(0) // increment to force MainPreview reconnect on mode switch

const showConfigPanel = ref(false)

function onToggleSettings() {
  showConfigPanel.value = !showConfigPanel.value
}

function onSelectSource(source: CaptureSource) {
  selectedSource.value = source
}

function onModeChange(mode: 'server' | 'client') {
  if (mode === appMode.value) return
  appMode.value = mode
  if (mode === 'server') {
    // Switching back to server mode — force preview reconnect
    serverModeKey.value++
  }
}

async function onStopStream(sourceId: string) {
  try {
    await pipelineStore.stopStream(sourceId)
  } catch (e) {
    console.error('Failed to stop stream:', e)
  }
}

// Client mode event handlers
function onSelectStream(streamId: string) {
  selectedStreamId.value = streamId
}

async function onDisconnectStream(streamId: string) {
  await rtspClientStore.disconnect(streamId)
}

async function onDeleteStream(streamId: string) {
  await rtspClientStore.disconnect(streamId)
}

async function onReconnectStream(streamId: string) {
  const status = rtspClientStore.streams.value[streamId]
  if (!status) return
  // Disconnect then reconnect with same params
  await rtspClientStore.disconnect(streamId)
  try {
    const newId = await rtspClientStore.connect(status.name, status.url, status.protocol)
    selectedStreamId.value = newId
  } catch (e) {
    console.error('Reconnect failed:', e)
  }
}

// Auto-sync RTSP client resolution to WS remote client for coordinate mapping
watch(
  () => rtspClientStore.streams.value,
  (streams) => {
    if (!wsRemoteStore.status.value.is_connected) return
    // Find the first connected stream with resolution info
    for (const streamId in streams) {
      const s = streams[streamId]
      if (s.state === 'Connected' && s.resolution) {
        const [w, h] = s.resolution
        wsRemoteStore.setResolution(w, h)
        break
      }
    }
  },
  { deep: true }
)
</script>

<template>
  <div class="app" :class="{ 'config-visible': showConfigPanel, 'client-mode': appMode === 'client' }" :style="{ gridTemplateColumns: showConfigPanel ? '320px 1fr 340px' : '320px 1fr 0px' }">
    <TopBar
      :settings-visible="showConfigPanel"
      :app-mode="appMode"
      @toggle-settings="onToggleSettings"
      @mode-change="onModeChange"
    />

    <!-- Server Mode -->
    <template v-if="appMode === 'server'">
      <SourceList
        :selected-source-id="selectedSource?.id ?? null"
        @select-source="onSelectSource"
      />
      <MainPreview
        :key="serverModeKey"
        :selected-source="selectedSource"
        :pipeline-status="selectedSource ? pipelineStore.pipelines.value[selectedSource.id] : undefined"
        @stop-stream="onStopStream"
      />
      <ConfigPanel :selected-source-id="selectedSource?.id ?? null" :visible="showConfigPanel" />
    </template>

    <!-- Client Mode -->
    <template v-if="appMode === 'client'">
      <RtspStreamList
        :selected-stream-id="selectedStreamId"
        @select-stream="onSelectStream"
        @disconnect-stream="onDisconnectStream"
        @reconnect-stream="onReconnectStream"
        @delete-stream="onDeleteStream"
      />
      <RtspPreview :stream-id="selectedStreamId" />
      <RemoteControlClient />
    </template>

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

/* Client mode uses purple accent */
.app.client-mode {
  --mode-accent: var(--accent-purple);
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
