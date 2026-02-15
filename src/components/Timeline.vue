<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMemoryStore } from '../stores/memoryStore'
import type { Memory } from '../types/memory'

const memoryStore = useMemoryStore()
const items = ref<Memory[]>([])

onMounted(async () => {
  items.value = await memoryStore.fetchTimeline()
})

function preview(content: string) {
  return content.length > 120 ? content.slice(0, 120) + '…' : content
}

function formatDate(created: string) {
  return created.slice(0, 16).replace('T', ' ')
}
</script>

<template>
  <div class="timeline">
    <h2 class="panel-title">时间轴</h2>
    <p v-if="memoryStore.loading" class="loading">加载中…</p>
    <p v-else-if="memoryStore.error" class="error">{{ memoryStore.error }}</p>
    <div v-else class="timeline-list">
      <div
        v-for="m in items"
        :key="m.id"
        class="timeline-item"
        @click="memoryStore.setCurrentMemory(m)"
      >
        <div class="timeline-dot" />
        <div class="timeline-content">
          <time class="timeline-date">{{ formatDate(m.created_at) }}</time>
          <p class="timeline-preview">{{ preview(m.content) }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.timeline {
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
.timeline-list {
  flex: 1;
  overflow-y: auto;
  padding-left: 0.5rem;
}
.timeline-item {
  position: relative;
  padding-bottom: 1rem;
  cursor: pointer;
}
.timeline-item::before {
  content: '';
  position: absolute;
  left: 5px;
  top: 12px;
  bottom: -0.5rem;
  width: 1px;
  background: #ddd;
}
.timeline-item:last-child::before {
  display: none;
}
.timeline-dot {
  position: absolute;
  left: 0;
  top: 4px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #24c8db;
}
.timeline-content {
  margin-left: 1.25rem;
}
.timeline-date {
  display: block;
  font-size: 0.75rem;
  color: #666;
  margin-bottom: 4px;
}
.timeline-preview {
  margin: 0;
  font-size: 0.875rem;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.timeline-item:hover .timeline-preview {
  color: #24c8db;
}
</style>
