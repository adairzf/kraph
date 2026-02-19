<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useMemoryStore } from '../stores/memoryStore'
import type { Memory } from '../types/memory'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const memoryStore = useMemoryStore()
const list = computed(() => memoryStore.memories)

onMounted(() => {
  memoryStore.fetchMemories()
})

function selectMemory(m: Memory) {
  memoryStore.setCurrentMemory(m)
}

function preview(content: string) {
  return content.length > 80 ? content.slice(0, 80) + '…' : content
}
</script>

<template>
  <div class="memory-list">
    <h2 class="panel-title">{{ t('memoryList.title') }}</h2>
    <p v-if="memoryStore.loading" class="loading">{{ t('memoryList.loading') }}</p>
    <p v-else-if="memoryStore.error" class="error">{{ memoryStore.error }}</p>
    <ul v-else class="list">
      <li
        v-for="m in list"
        :key="m.id"
        class="item"
        :class="{ active: memoryStore.currentMemory?.id === m.id }"
        @click="selectMemory(m)"
      >
        <span class="date">{{ m.created_at.slice(0, 16) }}</span>
        <span class="preview">{{ preview(m.content) }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.memory-list {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.panel-title {
  margin: 0 0 0.75rem 0;
  font-size: 1rem;
  font-weight: 600;
}
.loading,
.error {
  margin: 0.5rem 0;
  font-size: 0.875rem;
}
.error {
  color: var(--color-error, #c00);
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  flex: 1;
}
.item {
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 2px;
}
.item:hover {
  background: rgba(0, 0, 0, 0.06);
}
.item.active {
  background: rgba(0, 0, 0, 0.08);
}
.date {
  display: block;
  font-size: 0.75rem;
  color: #666;
  margin-bottom: 2px;
}
.preview {
  font-size: 0.875rem;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
