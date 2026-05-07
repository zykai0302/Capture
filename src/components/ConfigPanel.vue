<script setup lang="ts">
import { ref } from 'vue'
import EncodingConfig from './EncodingConfig.vue'
import RemoteControl from './RemoteControl.vue'
import NetworkConfig from './NetworkConfig.vue'

defineProps<{
  selectedSourceId: string | null
  visible: boolean
}>()

type ConfigTab = 'encoding' | 'remote' | 'network'
const activeTab = ref<ConfigTab>('encoding')
</script>

<template>
  <aside v-show="visible" class="panel-right">
    <div class="config-tabs">
      <button class="config-tab" :class="{ active: activeTab === 'encoding' }" @click="activeTab = 'encoding'">编码配置</button>
      <button class="config-tab" :class="{ active: activeTab === 'remote' }" @click="activeTab = 'remote'">反控设置</button>
      <button class="config-tab" :class="{ active: activeTab === 'network' }" @click="activeTab = 'network'">网络</button>
    </div>

    <div class="config-content">
      <EncodingConfig v-if="activeTab === 'encoding'" :source-id="selectedSourceId" />
      <RemoteControl v-if="activeTab === 'remote'" />
      <NetworkConfig v-if="activeTab === 'network'" />
    </div>
  </aside>
</template>

<style scoped>
.panel-right {
  grid-area: right;
  background: var(--bg-primary);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.config-tabs {
  display: flex;
  padding: 12px 16px 0;
  gap: 0;
  flex-shrink: 0;
}

.config-tab {
  flex: 1;
  padding: 10px 0;
  font-size: 11px;
  font-weight: 600;
  text-align: center;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.config-tab.active {
  color: var(--accent-cyan);
  border-bottom-color: var(--accent-cyan);
}

.config-tab:hover:not(.active) {
  color: var(--text-secondary);
}

.config-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
</style>
