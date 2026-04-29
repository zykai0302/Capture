<script setup lang="ts">
import { ref, computed, inject } from 'vue'
import type { useRemoteControl } from '../composables'

const { status, start, stop } = inject<ReturnType<typeof useRemoteControl>>('remote')!

const wsPort = ref(9001)
const wsPassword = ref('')
const enableRemote = computed({
  get: () => status.value.is_running,
  set: (val: boolean) => {
    if (val) {
      start(wsPort.value, wsPassword.value)
    } else {
      stop()
    }
  }
})

const capabilities = [
  { key: 'move', label: '鼠标移动', icon: '\u2191', enabled: true },
  { key: 'click', label: '鼠标点击', icon: '\u232B', enabled: true },
  { key: 'scroll', label: '鼠标滚动', icon: '\u21F5', enabled: true },
  { key: 'keyboard', label: '键盘输入', icon: '\u21E7', enabled: true },
  { key: 'combo', label: '组合快捷键', icon: '\u2326', enabled: true },
  { key: 'drag', label: '拖拽操作', icon: '\u21BA', enabled: true },
]
</script>

<template>
  <div class="remote-control-config">
    <div class="config-section">
      <div class="config-section-title">反控服务</div>
      <div class="config-row">
        <span class="config-label">启用反控</span>
        <div class="config-value">
          <div class="toggle-switch" :class="{ active: enableRemote }" @click="enableRemote = !enableRemote"></div>
        </div>
      </div>
      <div class="config-row">
        <span class="config-label">WebSocket 端口</span>
        <div class="config-value">
          <input type="number" class="custom-input" v-model.number="wsPort">
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">连接状态</div>
      <div class="remote-control-panel">
        <div class="rc-status">
          <div class="rc-status-left">
            <span class="rc-indicator" :class="status.is_running ? 'connected' : 'disconnected'"></span>
            <span class="rc-status-text" :style="{ color: status.is_running ? 'var(--accent-green)' : 'var(--text-muted)' }">
              {{ status.is_running ? '已连接' : '未启动' }}
            </span>
          </div>
          <span class="rc-clients">{{ status.client_count }} 客户端</span>
        </div>
        <div class="rc-capabilities">
          <div
            v-for="cap in capabilities"
            :key="cap.key"
            class="rc-cap-item"
            :class="status.is_running && cap.enabled ? 'enabled' : 'disabled'"
          >
            <span class="cap-icon">{{ cap.icon }}</span>
            {{ cap.label }}
          </div>
        </div>
      </div>
    </div>

    <div class="config-section">
      <div class="config-section-title">安全设置</div>
      <div class="config-row">
        <span class="config-label">反控密码</span>
        <div class="config-value">
          <input type="password" class="custom-input" v-model="wsPassword" style="width:110px;">
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

.remote-control-panel {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px;
}

.rc-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
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

.rc-clients {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

.rc-capabilities {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}

.rc-cap-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  padding: 6px 8px;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
}

.rc-cap-item .cap-icon {
  font-size: 12px;
}

.rc-cap-item.enabled .cap-icon {
  color: var(--accent-green);
}

.rc-cap-item.disabled .cap-icon {
  color: var(--text-muted);
}
</style>
