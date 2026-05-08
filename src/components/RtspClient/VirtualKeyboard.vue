<script setup lang="ts">
const props = defineProps<{
  enabled: boolean
}>()

const emit = defineEmits<{
  keyPress: [key: string, modifiers: string[]]
  keyCombo: [keys: string[]]
}>()

const keyboardRows = [
  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
  ['Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'],
  ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L'],
  ['Z', 'X', 'C', 'V', 'B', 'N', 'M'],
]

const specialKeys = [
  { label: 'Enter', key: 'Return' },
  { label: 'Esc', key: 'Escape' },
  { label: 'Tab', key: 'Tab' },
  { label: 'Space', key: 'space' },
  { label: 'Backspace', key: 'BackSpace' },
  { label: 'Delete', key: 'Delete' },
]

const shortcuts = [
  { label: 'Ctrl+C', keys: ['Ctrl', 'C'] },
  { label: 'Ctrl+V', keys: ['Ctrl', 'V'] },
  { label: 'Ctrl+Z', keys: ['Ctrl', 'Z'] },
  { label: 'Ctrl+A', keys: ['Ctrl', 'A'] },
  { label: 'Ctrl+S', keys: ['Ctrl', 'S'] },
  { label: 'Ctrl+X', keys: ['Ctrl', 'X'] },
  { label: 'Alt+F4', keys: ['Alt', 'F4'] },
  { label: 'Ctrl+Alt+Del', keys: ['Ctrl', 'Alt', 'Delete'] },
  { label: 'Win+D', keys: ['Super', 'd'] },
  { label: 'Ctrl+Shift+Esc', keys: ['Ctrl', 'Shift', 'Escape'] },
]

function onKeyPress(key: string) {
  if (!props.enabled) return
  emit('keyPress', key, [])
}

function onShortcut(keys: string[]) {
  if (!props.enabled) return
  emit('keyCombo', keys)
}
</script>

<template>
  <div class="virtual-keyboard" :class="{ disabled: !enabled }">
    <div class="kb-section-title">虚拟键盘</div>
    <div class="kb-rows">
      <div v-for="(row, ri) in keyboardRows" :key="ri" class="kb-row">
        <button
          v-for="key in row"
          :key="key"
          class="kb-key"
          :disabled="!enabled"
          @click="onKeyPress(key)"
        >{{ key }}</button>
      </div>
    </div>
    <div class="kb-special">
      <button
        v-for="sk in specialKeys"
        :key="sk.key"
        class="kb-key kb-key-wide"
        :disabled="!enabled"
        @click="onKeyPress(sk.key)"
      >{{ sk.label }}</button>
    </div>

    <div class="kb-section-title" style="margin-top: 12px">快捷键</div>
    <div class="kb-shortcuts">
      <button
        v-for="sc in shortcuts"
        :key="sc.label"
        class="kb-shortcut"
        :disabled="!enabled"
        @click="onShortcut(sc.keys)"
      >{{ sc.label }}</button>
    </div>
  </div>
</template>

<style scoped>
.virtual-keyboard {
  padding: 12px;
}

.virtual-keyboard.disabled {
  opacity: 0.4;
  pointer-events: none;
}

.kb-section-title {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.kb-rows {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: center;
}

.kb-row {
  display: flex;
  gap: 3px;
}

.kb-key {
  min-width: 26px;
  height: 28px;
  padding: 0 6px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.kb-key:hover:not(:disabled) {
  background: var(--bg-secondary);
  border-color: var(--accent-purple);
}

.kb-key:active:not(:disabled) {
  background: rgba(180, 77, 255, 0.2);
  transform: scale(0.95);
}

.kb-key:disabled {
  cursor: not-allowed;
}

.kb-key-wide {
  min-width: auto;
  padding: 0 10px;
  font-size: 10px;
}

.kb-special {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 6px;
  justify-content: center;
}

.kb-shortcuts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.kb-shortcut {
  padding: 6px 8px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-secondary);
  font-size: 10px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  text-align: center;
}

.kb-shortcut:hover:not(:disabled) {
  background: var(--bg-secondary);
  border-color: var(--accent-purple);
  color: var(--text-primary);
}

.kb-shortcut:active:not(:disabled) {
  background: rgba(180, 77, 255, 0.2);
  transform: scale(0.95);
}

.kb-shortcut:disabled {
  cursor: not-allowed;
}
</style>
