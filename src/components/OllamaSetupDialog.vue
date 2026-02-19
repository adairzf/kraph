<script setup lang="ts">
import { ref, nextTick, onUnmounted } from 'vue'
import { listen, type Event as TauriEvent } from '@tauri-apps/api/event'
import { runOllamaSetup } from '../utils/tauriApi'

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

async function openAndStart() {
  // 清理上一次的监听器
  cleanup()

  // 重置状态
  visible.value = true
  logs.value = []
  isRunning.value = true
  isDone.value = false
  isSuccess.value = false
  counter = 0

  // 先注册事件监听，再触发后端命令，避免错过事件
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

  // 启动后端初始化流程
  try {
    await runOllamaSetup()
  } catch (e) {
    logs.value.push({
      id: ++counter,
      message: `初始化出错: ${e instanceof Error ? e.message : String(e)}`,
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

defineExpose({ openAndStart })
</script>

<template>
  <el-dialog
    v-model="visible"
    title="Ollama 一键初始化"
    width="580px"
    :close-on-click-modal="false"
    :close-on-press-escape="!isRunning"
    @closed="cleanup"
  >
    <div class="setup-body">
      <!-- 顶部状态提示 -->
      <div class="setup-status" :class="{ running: isRunning, success: isDone && isSuccess, failed: isDone && !isSuccess }">
        <template v-if="isRunning">
          <span class="spinner-icon">◌</span>
          正在自动检查并配置 Ollama 环境，请稍候...
        </template>
        <template v-else-if="isDone && isSuccess">
          🎉 初始化完成！Ollama 已就绪，可以开始使用。
        </template>
        <template v-else-if="isDone">
          ⚠️ 初始化未完全成功，请查看日志了解详情。
        </template>
        <template v-else>
          点击"开始初始化"自动完成所有配置步骤。
        </template>
      </div>

      <!-- 步骤说明（仅在未开始时显示） -->
      <div v-if="!isRunning && !isDone" class="setup-steps">
        <p>将依次执行以下步骤（已完成的步骤自动跳过）：</p>
        <ol>
          <li>检查 Ollama 是否已安装（未安装则下载安装程序）</li>
          <li>检查 Ollama 服务是否运行（未运行则自动启动）</li>
          <li>检查所需模型是否已下载（未下载则自动拉取）</li>
        </ol>
      </div>

      <!-- 日志输出区域 -->
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
          重新初始化
        </el-button>
        <el-button
          v-if="!isRunning && !isDone"
          type="primary"
          @click="openAndStart"
        >
          开始初始化
        </el-button>
        <el-button @click="handleClose" :type="isDone && isSuccess ? 'primary' : 'default'">
          {{ isDone ? '完成' : '关闭' }}
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
  background: #f4f4f5;
  color: #606266;
  display: flex;
  align-items: center;
  gap: 8px;
  line-height: 1.5;
}

.setup-status.running {
  background: #ecf5ff;
  color: #409eff;
}

.setup-status.success {
  background: #f0f9eb;
  color: #67c23a;
}

.setup-status.failed {
  background: #fdf6ec;
  color: #e6a23c;
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
  color: #606266;
  background: #f9f9f9;
  border-radius: 6px;
  padding: 10px 14px;
}

.setup-steps p {
  margin: 0 0 6px 0;
  font-weight: 500;
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
