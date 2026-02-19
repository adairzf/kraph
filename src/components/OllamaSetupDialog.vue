<script setup lang="ts">
import { ref, nextTick, onUnmounted } from 'vue'
import { listen, type Event as TauriEvent } from '@tauri-apps/api/event'
import { runOllamaSetup } from '../utils/tauriApi'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface LogEntry {
  id: number
  message: string
  status: 'running' | 'success' | 'error' | 'warning' | 'info'
}

const visible = ref(false)
const logs = ref<LogEntry[]>([])
const isRunning = ref(false)
const isDone = ref(false)
const isSuccess = ref(false)
const logContainer = ref<HTMLElement | null>(null)

let unlistenLog: (() => void) | null = null
let unlistenDone: (() => void) | null = null
let counter = 0

function statusIcon(status: string): string {
  switch (status) {
    case 'running': return '⏳'
    case 'success': return '✅'
    case 'error': return '❌'
    case 'warning': return '⚠️'
    default: return '📌'
  }
}

function cleanup() {
  unlistenLog?.()
  unlistenLog = null
  unlistenDone?.()
  unlistenDone = null
}

function open() {
  cleanup()
  visible.value = true
  logs.value = []
  isRunning.value = false
  isDone.value = false
  isSuccess.value = false
  counter = 0
}

async function openAndStart() {
  // Clean up listeners from the previous run
  cleanup()

  // Reset state
  visible.value = true
  logs.value = []
  isRunning.value = true
  isDone.value = false
  isSuccess.value = false
  counter = 0

  // Register event listeners before invoking the backend command to avoid missing early events
  unlistenLog = await listen<{ message: string; status: string }>(
    'ollama-setup-log',
    (event: TauriEvent<{ message: string; status: string }>) => {
      logs.value.push({
        id: ++counter,
        message: event.payload.message,
        status: event.payload.status as LogEntry['status'],
      })
      nextTick(() => {
        if (logContainer.value) {
          logContainer.value.scrollTop = logContainer.value.scrollHeight
        }
      })
    },
  )

  unlistenDone = await listen<{ success: boolean }>(
    'ollama-setup-done',
    (event: TauriEvent<{ success: boolean }>) => {
      isRunning.value = false
      isDone.value = true
      isSuccess.value = event.payload.success ?? false
      cleanup()
    },
  )

  try {
    await runOllamaSetup()
  } catch (e) {
    logs.value.push({
      id: ++counter,
      message: t('ollamaSetup.errors.initFailed') + (e instanceof Error ? e.message : String(e)),
      status: 'error',
    })
    isRunning.value = false
    isDone.value = true
    isSuccess.value = false
    cleanup()
  }
}

function handleClose() {
  visible.value = false
  cleanup()
}

onUnmounted(() => {
  cleanup()
})

defineExpose({ open, openAndStart })
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="t('ollamaSetup.title')"
    width="580px"
    :close-on-click-modal="false"
    :close-on-press-escape="!isRunning"
    @closed="cleanup"
  >
    <div class="setup-body">
      <div class="setup-status" :class="{ running: isRunning, success: isDone && isSuccess, failed: isDone && !isSuccess }">
        <template v-if="isRunning">
          <span class="spinner-icon">◌</span>
          {{ t('ollamaSetup.status.running') }}
        </template>
        <template v-else-if="isDone && isSuccess">
          {{ t('ollamaSetup.status.success') }}
        </template>
        <template v-else-if="isDone">
          {{ t('ollamaSetup.status.failed') }}
        </template>
        <template v-else>
          {{ t('ollamaSetup.status.idle') }}
        </template>
      </div>

      <div v-if="!isRunning && !isDone" class="setup-steps">
        <p>{{ t('ollamaSetup.steps.intro') }}</p>
        <ol>
          <li>{{ t('ollamaSetup.steps.checkInstall') }}</li>
          <li>{{ t('ollamaSetup.steps.checkService') }}</li>
          <li>{{ t('ollamaSetup.steps.checkModel') }}</li>
        </ol>
      </div>

      <!-- Log output area -->
      <div v-if="logs.length > 0 || isRunning" class="log-container" ref="logContainer">
        <div
          v-for="entry in logs"
          :key="entry.id"
          class="log-entry"
          :class="`log-${entry.status}`"
        >
          <span class="log-icon">{{ statusIcon(entry.status) }}</span>
          <span class="log-message">{{ entry.message }}</span>
        </div>
        <div v-if="isRunning" class="log-entry log-running log-cursor">
          <span class="spinner-text">▋</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button
          v-if="isDone && !isSuccess"
          type="primary"
          @click="openAndStart"
        >
          {{ t('ollamaSetup.buttons.retry') }}
        </el-button>
        <el-button
          v-if="!isRunning && !isDone"
          type="primary"
          @click="openAndStart"
        >
          {{ t('ollamaSetup.buttons.start') }}
        </el-button>
        <el-button @click="handleClose" :type="isDone && isSuccess ? 'primary' : 'default'">
          {{ isDone ? t('ollamaSetup.buttons.done') : t('ollamaSetup.buttons.close') }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<style scoped>
.setup-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.setup-status {
  padding: 10px 14px;
  border-radius: 6px;
  font-size: 14px;
  background: var(--bg4);
  color: var(--text-muted);
  border: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 8px;
  line-height: 1.5;
}

.setup-status.running {
  background: rgba(59, 130, 246, 0.1);
  color: #60a5fa;
  border-color: rgba(59, 130, 246, 0.25);
}

.setup-status.success {
  background: rgba(52, 211, 153, 0.1);
  color: var(--green);
  border-color: rgba(52, 211, 153, 0.25);
}

.setup-status.failed {
  background: rgba(251, 146, 60, 0.1);
  color: var(--orange);
  border-color: rgba(251, 146, 60, 0.25);
}

.spinner-icon {
  display: inline-block;
  animation: spin 1.2s linear infinite;
  font-size: 16px;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.setup-steps {
  font-size: 13px;
  color: var(--text-muted);
  background: var(--bg4);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 14px;
}

.setup-steps p {
  margin: 0 0 6px 0;
  font-weight: 500;
  color: var(--text);
}

.setup-steps ol {
  margin: 0;
  padding-left: 20px;
}

.setup-steps li {
  margin: 4px 0;
  line-height: 1.5;
}

.log-container {
  background: #1e1e2e;
  border-radius: 8px;
  padding: 12px 14px;
  max-height: 280px;
  overflow-y: auto;
  font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.7;
  scroll-behavior: smooth;
}

.log-container::-webkit-scrollbar {
  width: 4px;
}

.log-container::-webkit-scrollbar-track {
  background: transparent;
}

.log-container::-webkit-scrollbar-thumb {
  background: #444466;
  border-radius: 2px;
}

.log-entry {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 1px 0;
}

.log-icon {
  flex-shrink: 0;
  width: 18px;
}

.log-message {
  flex: 1;
  word-break: break-all;
}

.log-running .log-message {
  color: #89b4fa;
}

.log-success .log-message {
  color: #a6e3a1;
}

.log-error .log-message {
  color: #f38ba8;
}

.log-warning .log-message {
  color: #fab387;
}

.log-info .log-message {
  color: #cdd6f4;
}

.log-cursor {
  color: #89b4fa;
}

.spinner-text {
  display: inline-block;
  animation: blink 1s step-start infinite;
  color: #89b4fa;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
