<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useMemoryStore } from '../stores/memoryStore'
import { useGraphStore } from '../stores/graphStore'
import type { Memory } from '../types/memory'
import { marked } from 'marked'
import { ElMessageBox, ElMessage } from 'element-plus'
import { useI18n } from 'vue-i18n'

const { t, te } = useI18n()

// Returns [translatedMessage, isKnownBackendCode]
function parseBackendError(raw: string): [string, boolean] {
  try {
    const parsed = JSON.parse(raw)
    if (parsed.code && te(parsed.code)) {
      return [t(parsed.code, parsed), true]
    }
  } catch { /* not JSON */ }
  return [raw, false]
}

const memoryStore = useMemoryStore()
const graphStore = useGraphStore()
const content = ref('')
const previewMode = ref(false)
const saving = ref(false)
const deleting = ref(false)
type SaveProgressStep = {
  message: string
  status: 'info' | 'running' | 'success' | 'warning' | 'error' | 'skipped' | 'done'
}
const saveProgress = ref<SaveProgressStep[]>([])

watch(
  () => memoryStore.currentMemory,
  (m: Memory | null) => {
    content.value = m?.content ?? ''
    if (m) {
      previewMode.value = true
    }
  },
  { immediate: true }
)

const html = () => (previewMode.value ? marked(content.value) : '')

async function handleSave() {
  if (!memoryStore.currentMemory || !content.value.trim()) return
  if (content.value.trim() === memoryStore.currentMemory.content.trim()) {
    ElMessage.info(t('editorPanel.noChanges'))
    return
  }
  saving.value = true
  saveProgress.value = []

  const unlisten = await listen<{ code: string; params: Record<string, unknown>; status: string }>('memory-save-progress', (event) => {
    const { code, params } = event.payload
    const message = te(code) ? t(code, params) : code
    saveProgress.value.push({
      message,
      status: event.payload.status as SaveProgressStep['status'],
    })
  })

  try {
    await nextTick()
    await new Promise<void>((resolve) => {
      window.requestAnimationFrame(() => resolve())
    })
    await memoryStore.updateMemoryContent(memoryStore.currentMemory.id, content.value.trim())
    await graphStore.fetchGraph()
    ElMessage.success(t('editorPanel.saveSuccess'))
    await new Promise((resolve) => setTimeout(resolve, 1200))
    saveProgress.value = []
  } catch (e) {
    const raw = e instanceof Error ? e.message : String(e)
    const [msg, isKnown] = parseBackendError(raw)
    // Known backend errors (entity extraction) are self-contained; others get the "saveFailed" prefix
    ElMessage.error(isKnown ? msg : t('editorPanel.saveFailed') + msg)
    saveProgress.value = []
  } finally {
    unlisten()
    saving.value = false
  }
}

async function handleDelete() {
  if (!memoryStore.currentMemory) return
  
  try {
    await ElMessageBox.confirm(
      t('editorPanel.deleteConfirm.message'),
      t('editorPanel.deleteConfirm.title'),
      {
        confirmButtonText: t('editorPanel.deleteConfirm.confirm'),
        cancelButtonText: t('editorPanel.deleteConfirm.cancel'),
        type: 'warning',
        confirmButtonClass: 'el-button--danger'
      }
    )
    
    deleting.value = true
    try {
      await memoryStore.deleteMemoryById(memoryStore.currentMemory!.id)
      await graphStore.fetchGraph()
      ElMessage.success(t('editorPanel.deleteSuccess'))
    } catch (e) {
      ElMessage.error(t('editorPanel.deleteFailed') + (e instanceof Error ? e.message : String(e)))
    } finally {
      deleting.value = false
    }
  } catch {
    // user cancelled delete
  }
}
</script>

<template>
  <div class="editor-panel">
    <div class="toolbar">
      <div class="tabs">
        <button
          type="button"
          class="tab"
          :class="{ active: previewMode }"
          @click="previewMode = true"
        >
          {{ t('editorPanel.preview') }}
        </button>
        <button
          type="button"
          class="tab"
          :class="{ active: !previewMode }"
          @click="previewMode = false"
        >
          {{ t('editorPanel.edit') }}
        </button>
      </div>
      <div v-if="memoryStore.currentMemory" class="actions">
        <button
          type="button"
          class="btn btn-save"
          :disabled="saving || !content.trim()"
          @click="handleSave"
        >
          {{ saving ? t('editorPanel.saving') : t('editorPanel.save') }}
        </button>
        <button
          type="button"
          class="btn btn-delete"
          :disabled="deleting"
          @click="handleDelete"
        >
          {{ deleting ? t('editorPanel.deleting') : t('editorPanel.delete') }}
        </button>
      </div>
    </div>
    <div v-if="!memoryStore.currentMemory" class="empty">
      {{ t('editorPanel.empty') }}
    </div>
    <template v-else>
      <textarea
        v-if="!previewMode"
        v-model="content"
        class="textarea"
        :placeholder="t('editorPanel.placeholder')"
      />
      <div
        v-else
        class="preview-content"
        v-html="html()"
      />
      <div v-if="saveProgress.length > 0" class="save-progress">
        <div
          v-for="(step, i) in saveProgress"
          :key="i"
          class="progress-step"
          :class="`step-${step.status}`"
        >
          <span class="step-icon">
            <span v-if="step.status === 'running'" class="spinner">⏳</span>
            <span v-else-if="step.status === 'success'">✅</span>
            <span v-else-if="step.status === 'done'">🎉</span>
            <span v-else-if="step.status === 'warning'">⚠️</span>
            <span v-else-if="step.status === 'error'">❌</span>
            <span v-else-if="step.status === 'skipped'">⏭️</span>
            <span v-else>ℹ️</span>
          </span>
          <span class="step-message">{{ step.message }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.editor-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.tabs { display: flex; gap: 3px; }
.tab {
  padding: 5px 12px;
  border: 1px solid var(--border);
  background: var(--bg4);
  color: var(--text-muted);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-family: inherit;
  transition: all 0.15s;
}
.tab:hover { color: var(--text); border-color: var(--border-hover); }
.tab.active {
  background: rgba(124, 92, 252, 0.12);
  border-color: rgba(124, 92, 252, 0.35);
  color: #a78bfa;
}
.actions { display: flex; gap: 6px; }
.btn {
  padding: 5px 12px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg4);
  color: var(--text-muted);
  cursor: pointer;
  font-size: 13px;
  font-family: inherit;
  transition: all 0.15s;
}
.btn:hover { color: var(--text); border-color: var(--border-hover); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-save {
  background: var(--grad);
  color: #fff;
  border-color: transparent;
  font-weight: 500;
}
.btn-save:hover:not(:disabled) { opacity: 0.88; }
.btn-delete {
  background: rgba(248, 113, 113, 0.1);
  color: var(--red);
  border-color: rgba(248, 113, 113, 0.25);
}
.btn-delete:hover:not(:disabled) { background: rgba(248, 113, 113, 0.18); }
.empty {
  color: var(--text-dim);
  padding: 32px;
  text-align: center;
  font-size: 13px;
}
.textarea {
  flex: 1;
  width: 100%;
  box-sizing: border-box;
  padding: 10px 12px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  resize: none;
  font-size: 13px;
  line-height: 1.6;
  border-radius: 8px;
  outline: none;
  font-family: inherit;
  transition: border-color 0.15s;
}
.textarea:focus { border-color: rgba(124, 92, 252, 0.5); }
.preview-content {
  flex: 1;
  padding: 10px 12px;
  overflow-y: auto;
  font-size: 14px;
  line-height: 1.65;
  color: var(--text);
}
.preview-content :deep(h1) { font-size: 18px; color: var(--text); margin-bottom: 8px; }
.preview-content :deep(h2) { font-size: 16px; color: var(--text); margin-bottom: 6px; }
.preview-content :deep(ul) { padding-left: 20px; color: var(--text-muted); }
.preview-content :deep(p) { color: var(--text-muted); margin-bottom: 8px; }

.save-progress {
  margin-top: 8px;
  padding: 8px;
  border: 1px solid var(--border);
  background: var(--bg3);
  border-radius: 8px;
  max-height: 150px;
  overflow-y: auto;
}
.progress-step {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-muted);
  padding: 4px 0;
}
.step-icon {
  display: inline-flex;
  width: 16px;
  justify-content: center;
}
.step-message {
  flex: 1;
}
.step-running { color: var(--text); }
.step-success,
.step-done { color: var(--green); }
.step-warning { color: #f59e0b; }
.step-error { color: var(--red); }
.preview-content :deep(code) {
  background: var(--bg4);
  border: 1px solid var(--border);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 12px;
}
</style>
