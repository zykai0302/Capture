<script setup lang="ts">
import { ref, computed, inject } from 'vue'
import type { useWsRemote } from '../../composables'
import VirtualKeyboard from './VirtualKeyboard.vue'

const wsRemote = inject<ReturnType<typeof useWsRemote>>('wsRemote')!

const wsUrl = ref('ws://192.168.1.100:9001')
const wsPassword = ref('')

const isConnected = computed(() => wsRemote.status.value.is_connected)

async function onConnect() {
  try {
    await wsRemote.connect(wsUrl.value, wsPassword.value || undefined)
  } catch (e) {
    console.error('Failed to connect:', e)
  }
}

async function onDisconnect() {
  try {
    await wsRemote.disconnect()
  } catch (e) {
    console.error('Failed to disconnect:', e)
  }
}

function onKeyPress(key: string, modifiers: string[]) {
  if (!wsRemote.status.value.is_connected) return
  // Use first stream as default - in real app, this would come from the selected stream
  wsRemote.sendKeyPress('default', key, modifiers)
}

function onKeyCombo(keys: string[]) {
  if (!wsRemote.status.value.is_connected) return
  wsRemote.sendKeyCombo('default', keys)
}

const capabilities = [
  { key: 'move', label: '鼠标移动', icon: '↕', enabled: true },
  { key: 'click', label: '鼠标点击', icon: '⌧', enabled: true },
  { key: 'scroll', label: '鼠标滚动', icon: '⇵', enabled: true },
  { key: 'drag', label: '拖拽操作', icon: '↺', enabled: true },
  { key: 'keyboard', label: '键盘输入', icon: '⇧', enabled: true },
  { key: 'combo', label: '组合快捷键', icon: '⌦', enabled: true },
]
</script>

<template>
  <div class="remote-control-client">
    <div class="config-section">
      <div class="config-section-title">反控连接</div>
      <div class="config-row">
        <span class="config-label">WebSocket 地址</span>
      </div>
      <input v-model="wsUrl" class="form-input font-mono" placeholder="ws://host:9001" :disabled="isConnected" />
      <div class="config-row" style="margin-top: 8px">
        <span class="config-label">密码</span>
      </div>
      <input v-model="wsPassword" type="password" class="form-input" placeholder="可选" :disabled="isConnected" />
      <button
        class="connect-btn"
        :class="isConnected ? 'btn-disconnect' : 'btn-connect'"
        @click="isConnected ? onDisconnect() : onConnect()"
      >
        {{ isConnected ? '断开连接' : '连接' }}
      </button>
    </div>

    <div class="config-section">
      <div class="config-section-title">连接状态</div>
      <div class="status-panel">
        <div class="rc-status">
          <div class="rc-status-left">
            <span class="rc-indicator" :class="isConnected ? 'connected' : 'disconnected'"></span>
            <span class="rc-status-text" :style="{ color: isConnected ? 'var(--accent-green)' : 'var(--text-muted)' }">
              {{ wsRemote.status.value.is_reconnecting ? '重连中...' : isConnected ? '已连接' : '未连接' }}
            </span>
          </div>
          <span v-if="isConnected" class="rc-url font-mono">{{ wsRemote.status.value.remote_url }}</span>
        </div>
        <div class="rc-capabilities">
          <div
            v-for="cap in capabilities"
            :key="cap.key"
            class="rc-cap-item"
            :class="isConnected && cap.enabled ? 'enabled' : 'disabled'"
          >
            <span class="cap-icon">{{ cap.icon }}</span>
            {{ cap.label }}
          </div>
        </div>
      </div>
    </div>

    <div class="keyboard-section">
      <VirtualKeyboard :enabled="isConnected" @key-press="onKeyPress" @key-combo="onKeyCombo" />
    </div>
  </div>
</template>

<style scoped>
.remote-control-client {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
  overflow-y: auto;
}

.config-section {
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
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
  padding: 4px 0;
}

.config-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.form-input {
  width: 100%;
  padding: 6px 10px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  transition: all 0.2s;
  box-sizing: border-box;
}

.form-input:focus {
  outline: none;
  border-color: var(--accent-purple);
  box-shadow: 0 0 0 2px rgba(180, 77, 255, 0.15);
}

.form-input:disabled {
  opacity: 0.5;
}

.connect-btn {
  width: 100%;
  margin-top: 10px;
  padding: 8px;
  font-size: 12px;
  font-weight: 600;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid;
}

.btn-connect {
  color: var(--accent-purple);
  background: rgba(180, 77, 255, 0.1);
  border-color: rgba(180, 77, 255, 0.3);
}

.btn-connect:hover {
  background: rgba(180, 77, 255, 0.2);
}

.btn-disconnect {
  color: var(--accent-red);
  background: rgba(255, 59, 48, 0.1);
  border-color: rgba(255, 59, 48, 0.3);
}

.btn-disconnect:hover {
  background: rgba(255, 59, 48, 0.2);
}

.status-panel {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px;
}

.rc-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.rc-status-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rc-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.rc-indicator.connected {
  background: var(--accent-green);
  box-shadow: 0 0 8px var(--accent-green);
}

.rc-indicator.disconnected {
  background: var(--text-muted);
}

.rc-status-text {
  font-size: 12px;
  font-weight: 500;
}

.rc-url {
  font-size: 9px;
  color: var(--text-muted);
  max-width: 120px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rc-capabilities {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.rc-cap-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10px;
  color: var(--text-secondary);
  padding: 5px 7px;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
}

.rc-cap-item .cap-icon {
  font-size: 11px;
}

.rc-cap-item.enabled .cap-icon {
  color: var(--accent-green);
}

.rc-cap-item.disabled .cap-icon {
  color: var(--text-muted);
}

.keyboard-section {
  flex: 1;
}

.font-mono { font-family: var(--font-mono); }
</style>
