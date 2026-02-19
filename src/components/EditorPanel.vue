<script setup lang="ts">
import { ref, watch } from 'vue'
import { useMemoryStore } from '../stores/memoryStore'
import { useGraphStore } from '../stores/graphStore'
import type { Memory } from '../types/memory'
import { marked } from 'marked'
import { ElMessageBox, ElMessage } from 'element-plus'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const memoryStore = useMemoryStore()
const graphStore = useGraphStore()
const content = ref('')
const previewMode = ref(false)
const saving = ref(false)
const deleting = ref(false)

watch(
  () => memoryStore.currentMemory,
  (m: Memory | null) => {
    content.value = m?.content ?? ''
  },
  { immediate: true }
)

const html = () => (previewMode.value ? marked(content.value) : '')

async function handleSave() {
  if (!memoryStore.currentMemory || !content.value.trim()) return
  saving.value = true
  try {
    await memoryStore.updateMemoryContent(memoryStore.currentMemory.id, content.value.trim())
    await graphStore.fetchGraph()
    ElMessage.success(t('editorPanel.saveSuccess'))
  } catch (e) {
    ElMessage.error(t('editorPanel.saveFailed') + (e instanceof Error ? e.message : String(e)))
  } finally {
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
    // user cancelled
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
          :class="{ active: !previewMode }"
          @click="previewMode = false"
        >
          {{ t('editorPanel.edit') }}
        </button>
        <button
          type="button"
          class="tab"
          :class="{ active: previewMode }"
          @click="previewMode = true"
        >
          {{ t('editorPanel.preview') }}
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
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}
.tabs {
  display: flex;
  gap: 4px;
}
.tab {
  padding: 0.35rem 0.75rem;
  border: 1px solid #ddd;
  background: #f9f9f9;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.875rem;
}
.tab.active {
  background: #fff;
  border-color: #24c8db;
  color: #24c8db;
}
.actions {
  display: flex;
  gap: 0.5rem;
}
.btn {
  padding: 0.35rem 0.75rem;
  border-radius: 4px;
  border: 1px solid #ccc;
  background: #f5f5f5;
  cursor: pointer;
  font-size: 0.875rem;
}
.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.btn-save {
  background: #24c8db;
  color: #fff;
  border-color: #24c8db;
}
.btn-save:hover:not(:disabled) {
  background: #1fa8c0;
}
.btn-delete {
  background: #ff4444;
  color: #fff;
  border-color: #ff4444;
}
.btn-delete:hover:not(:disabled) {
  background: #cc0000;
}
.empty {
  color: #888;
  padding: 2rem;
  text-align: center;
}
.textarea {
  flex: 1;
  width: 100%;
  box-sizing: border-box;
  padding: 0.75rem;
  border: none;
  resize: none;
  font-size: 0.9375rem;
  line-height: 1.5;
}
.preview-content {
  flex: 1;
  padding: 0.75rem;
  overflow-y: auto;
  font-size: 0.9375rem;
  line-height: 1.6;
}
.preview-content :deep(h1) { font-size: 1.25rem; }
.preview-content :deep(h2) { font-size: 1.1rem; }
.preview-content :deep(ul) { padding-left: 1.25rem; }
</style>
