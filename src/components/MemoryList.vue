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
  margin: 0 0 8px 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  padding: 0 4px;
}
.loading,
.error {
  margin: 8px 4px;
  font-size: 13px;
  color: var(--text-muted);
}
.error { color: var(--red); }
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  flex: 1;
}
.list::-webkit-scrollbar { width: 3px; }
.list::-webkit-scrollbar-track { background: transparent; }
.list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }
.item {
  padding: 7px 9px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 1px;
  transition: background 0.15s;
  border-left: 2px solid transparent;
}
.item:hover { background: var(--bg4); }
.item.active {
  background: var(--bg4);
  border-left-color: var(--accent);
}
.date {
  display: block;
  font-size: 11px;
  color: var(--text-dim);
  margin-bottom: 2px;
}
.preview {
  font-size: 13px;
  color: var(--text);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
