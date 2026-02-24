<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { ElMessageBox } from 'element-plus'
import { answerQuestion } from '../utils/tauriApi'
import { useOllamaStore } from '../stores/ollamaStore'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const question = ref('')
const answer = ref('')
const loading = ref(false)
const error = ref<string | null>(null)
const progressText = ref('')
const progressSummary = ref('')
const progressTimer = ref<number | null>(null)
const progressStep = ref(0)
const progressStartedAt = ref<number | null>(null)

const ollamaStore = useOllamaStore()

/** Returns true when the error message indicates an Ollama service failure. */
function isOllamaError(msg: string): boolean {
  const keywords = [
    'ollama', 'Ollama',
    'localhost:11434',
    '127.0.0.1:11434',
  ]
  return keywords.some((k) => msg.includes(k))
}

async function ask() {
  const q = question.value.trim()
  if (!q || loading.value) return
  loading.value = true
  error.value = null
  answer.value = ''
  startProgress()
  await nextTick()
  await new Promise<void>((resolve) => {
    window.setTimeout(() => resolve(), 0)
  })
  await new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve())
  })
  let success = false
  try {
    answer.value = await answerQuestion(q)
    success = true
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg

    if (isOllamaError(errMsg)) {
      try {
        await ElMessageBox.confirm(
          t('searchPanel.ollamaError.message'),
          t('searchPanel.ollamaError.title'),
          {
            confirmButtonText: t('searchPanel.ollamaError.confirm'),
            cancelButtonText: t('searchPanel.ollamaError.cancel'),
            type: 'warning',
          },
        )
        ollamaStore.requestSetup()
      } catch {
        // user dismissed the dialog
      }
    }
  } finally {
    loading.value = false
    stopProgress(success)
  }
}

function currentElapsedSeconds(): number {
  if (!progressStartedAt.value) return 0
  return Math.max(0, Math.floor((Date.now() - progressStartedAt.value) / 1000))
}

function updateProgressText() {
  const elapsed = currentElapsedSeconds()
  let phaseKey = 'searchPanel.progress.connecting'
  if (elapsed >= 4) phaseKey = 'searchPanel.progress.retrieving'
  if (elapsed >= 10) phaseKey = 'searchPanel.progress.generating'

  const stepMap: Record<string, number> = {
    'searchPanel.progress.connecting': 1,
    'searchPanel.progress.retrieving': 2,
    'searchPanel.progress.generating': 3,
  }
  progressStep.value = stepMap[phaseKey]
  progressText.value = `${t(phaseKey)} · ${elapsed}s`
}

function startProgress() {
  progressSummary.value = ''
  progressText.value = ''
  progressStep.value = 0
  progressStartedAt.value = Date.now()
  updateProgressText()

  if (progressTimer.value != null) {
    window.clearInterval(progressTimer.value)
  }
  progressTimer.value = window.setInterval(updateProgressText, 500)
}

function stopProgress(success: boolean) {
  if (progressTimer.value != null) {
    window.clearInterval(progressTimer.value)
    progressTimer.value = null
  }
  const elapsed = currentElapsedSeconds()
  progressStep.value = success ? 4 : Math.max(1, progressStep.value)
  progressSummary.value = success
    ? t('searchPanel.progress.done', { seconds: elapsed })
    : t('searchPanel.progress.failed', { seconds: elapsed })
  progressStartedAt.value = null
  progressText.value = ''
}
</script>

<template>
  <div class="search-panel">
    <h2 class="panel-title">{{ t('searchPanel.title') }}</h2>
    <p class="hint">{{ t('searchPanel.hint') }}</p>
    <div class="qa-form">
      <input
        v-model="question"
        type="text"
        class="question-input"
        :placeholder="t('searchPanel.placeholder')"
        @keyup.enter="ask"
      />
      <button
        type="button"
        class="btn-ask"
        :disabled="!question.trim()"
        @click="ask"
      >
        {{ loading ? t('searchPanel.thinking') : t('searchPanel.ask') }}
      </button>
    </div>
    <div
      v-if="loading || progressSummary"
      class="progress-box"
    >
      <div class="progress-track">
        <div
          class="progress-fill"
          :style="{ width: `${Math.min(100, progressStep * 25)}%` }"
        />
      </div>
      <p class="progress-text">{{ loading ? progressText : progressSummary }}</p>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <div v-else-if="answer" class="answer-box">
      {{ answer }}
    </div>
  </div>
</template>

<style scoped>
.search-panel { padding: 0; }
.panel-title {
  margin: 0 0 4px 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.hint {
  margin: 0 0 10px 0;
  font-size: 13px;
  color: var(--text-muted);
}
.qa-form {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}
.question-input {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}
.question-input::placeholder { color: var(--text-dim); }
.question-input:focus { border-color: rgba(124, 92, 252, 0.5); }
.btn-ask {
  padding: 7px 14px;
  border: none;
  background: var(--grad);
  color: #fff;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  transition: opacity 0.15s;
  white-space: nowrap;
}
.btn-ask:hover { opacity: 0.88; }
.btn-ask:disabled { opacity: 0.4; cursor: not-allowed; }
.progress-box {
  margin: 0 0 10px 0;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.progress-track {
  width: 100%;
  height: 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-dim) 18%, transparent);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  width: 0;
  border-radius: inherit;
  background: var(--grad);
  transition: width 0.3s ease;
}
.progress-text {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.error {
  color: var(--red);
  font-size: 13px;
  margin: 8px 0;
  padding: 8px 10px;
  background: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 6px;
}
.answer-box {
  padding: 12px;
  background: var(--bg3);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.65;
  white-space: pre-wrap;
  color: var(--text);
}
</style>
