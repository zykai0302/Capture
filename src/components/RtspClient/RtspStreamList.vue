<script setup lang="ts">
import { ref, inject } from 'vue'
import type { useRtspClient } from '../../composables'
import { getRtspClientStateLabel, isRtspClientConnected, isRtspClientDisconnected, type RtspClientState } from '../../types'

defineProps<{
  selectedStreamId: string | null
}>()

const emit = defineEmits<{
  selectStream: [streamId: string]
  disconnectStream: [streamId: string]
  reconnectStream: [streamId: string]
  deleteStream: [streamId: string]
}>()

const rtspClient = inject<ReturnType<typeof useRtspClient>>('rtspClient')!

const showAddForm = ref(false)
const newName = ref('')
const newUrl = ref('rtsp://')
const newProtocol = ref('tcp')
const newUsername = ref('')
const newPassword = ref('')

async function addStream() {
  if (!newName.value.trim() || !newUrl.value.trim()) return
  try {
    const streamId = await rtspClient.connect(
      newName.value.trim(),
      newUrl.value.trim(),
      newProtocol.value,
      newUsername.value.trim() || undefined,
      newPassword.value.trim() || undefined,
    )
    emit('selectStream', streamId)
    resetForm()
  } catch (e) {
    console.error('Failed to add RTSP stream:', e)
  }
}

function resetForm() {
  showAddForm.value = false
  newName.value = ''
  newUrl.value = 'rtsp://'
  newProtocol.value = 'tcp'
  newUsername.value = ''
  newPassword.value = ''
}

function getStatusClass(state: RtspClientState): string {
  if (typeof state === 'string') {
    switch (state) {
      case 'Connecting': return 'status-connecting'
      case 'Connected': return 'status-connected'
      case 'Disconnected': return 'status-disconnected'
      case 'Offline': return 'status-offline'
      default: return ''
    }
  }
  if (typeof state === 'object') {
    if ('Reconnecting' in state) return 'status-reconnecting'
    if ('Error' in state) return 'status-error'
  }
  return ''
}
</script>

<template>
  <div class="rtsp-stream-list">
    <div class="list-header">
      <span class="list-title">RTSP 流列表</span>
      <button class="add-btn" @click="showAddForm = !showAddForm">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        添加流
      </button>
    </div>

    <!-- Add Stream Form -->
    <div v-if="showAddForm" class="add-form">
      <div class="form-group">
        <label>名称</label>
        <input v-model="newName" class="form-input" placeholder="摄像头 1" />
      </div>
      <div class="form-group">
        <label>RTSP 地址</label>
        <input v-model="newUrl" class="form-input font-mono" placeholder="rtsp://..." />
      </div>
      <div class="form-group">
        <label>传输协议</label>
        <select v-model="newProtocol" class="form-input">
          <option value="tcp">TCP</option>
          <option value="udp">UDP</option>
        </select>
      </div>
      <div class="form-row">
        <div class="form-group flex-1">
          <label>用户名</label>
          <input v-model="newUsername" class="form-input" placeholder="可选" />
        </div>
        <div class="form-group flex-1">
          <label>密码</label>
          <input v-model="newPassword" type="password" class="form-input" placeholder="可选" />
        </div>
      </div>
      <div class="form-actions">
        <button class="btn-secondary" @click="resetForm">取消</button>
        <button class="btn-primary" @click="addStream">连接</button>
      </div>
    </div>

    <!-- Stream Cards -->
    <div class="stream-cards">
      <div
        v-for="(status, streamId) in rtspClient.streams.value"
        :key="streamId"
        class="stream-card"
        :class="{ selected: streamId === selectedStreamId }"
        @click="emit('selectStream', streamId)"
      >
        <div class="card-header">
          <span class="stream-name">{{ status.name }}</span>
          <span class="status-badge" :class="getStatusClass(status.state)">
            {{ getRtspClientStateLabel(status.state) }}
          </span>
        </div>
        <div class="card-url font-mono">{{ status.url }}</div>
        <div class="card-info" v-if="status.resolution">
          {{ status.resolution[0] }}x{{ status.resolution[1] }}
        </div>
        <div class="card-actions">
          <button v-if="isRtspClientConnected(status.state)" class="card-btn btn-disconnect" @click.stop="emit('disconnectStream', streamId)">断开</button>
          <button v-if="isRtspClientDisconnected(status.state)" class="card-btn btn-reconnect" @click.stop="emit('reconnectStream', streamId)">重连</button>
          <button class="card-btn btn-delete" @click.stop="emit('deleteStream', streamId)">删除</button>
        </div>
      </div>

      <div v-if="Object.keys(rtspClient.streams.value).length === 0" class="empty-state">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
          <rect x="2" y="3" width="20" height="14" rx="2"/><rect x="7" y="21" width="10" height="2" rx="1"/>
        </svg>
        <div class="empty-text">点击上方"添加流"按钮</div>
        <div class="empty-sub">连接 RTSP 摄像头或流媒体源</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rtsp-stream-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
}

.list-header {
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
}

.list-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  font-size: 11px;
  font-weight: 500;
  color: var(--accent-purple);
  background: rgba(180, 77, 255, 0.1);
  border: 1px solid rgba(180, 77, 255, 0.3);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
}

.add-btn:hover {
  background: rgba(180, 77, 255, 0.2);
}

.add-form {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-deep);
}

.form-group {
  margin-bottom: 8px;
}

.form-group label {
  display: block;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.form-input {
  width: 100%;
  padding: 6px 10px;
  background: var(--bg-secondary);
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

.form-row {
  display: flex;
  gap: 8px;
}

.flex-1 { flex: 1; }

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 10px;
}

.btn-primary {
  padding: 6px 14px;
  font-size: 11px;
  font-weight: 500;
  color: white;
  background: var(--accent-purple);
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.btn-secondary {
  padding: 6px 14px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.stream-cards {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.stream-card {
  padding: 10px 12px;
  margin-bottom: 6px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.2s;
}

.stream-card:hover {
  border-color: rgba(180, 77, 255, 0.4);
}

.stream-card.selected {
  border-color: var(--accent-purple);
  background: rgba(180, 77, 255, 0.08);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.stream-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.status-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 500;
}

.status-connecting { background: rgba(255, 170, 0, 0.15); color: #ffaa00; }
.status-connected { background: rgba(0, 255, 136, 0.15); color: var(--accent-green); }
.status-reconnecting { background: rgba(255, 170, 0, 0.15); color: #ffaa00; }
.status-disconnected { background: rgba(102, 102, 102, 0.15); color: var(--text-muted); }
.status-error { background: rgba(255, 59, 48, 0.15); color: var(--accent-red); }
.status-offline { background: rgba(102, 102, 102, 0.1); color: var(--text-muted); opacity: 0.6; }

.card-url {
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}

.card-info {
  font-size: 10px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.card-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.card-btn {
  padding: 3px 8px;
  font-size: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  background: transparent;
  color: var(--text-secondary);
  transition: all 0.2s;
}

.btn-disconnect:hover { color: var(--accent-red); border-color: var(--accent-red); }
.btn-reconnect { color: var(--accent-purple); border-color: rgba(180, 77, 255, 0.3); }
.btn-reconnect:hover { border-color: var(--accent-purple); }
.btn-delete:hover { color: var(--accent-red); border-color: var(--accent-red); }

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-muted);
}

.empty-text {
  font-size: 13px;
  margin-top: 12px;
}

.empty-sub {
  font-size: 11px;
  opacity: 0.5;
  margin-top: 4px;
}

.font-mono { font-family: var(--font-mono); }
</style>
