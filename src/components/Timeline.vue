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
    <h2 class="panel-title">Timeline</h2>
    <p v-if="memoryStore.loading" class="loading">Loading…</p>
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
  margin: 0 0 10px 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
}
.loading,
.error {
  margin: 8px 0;
  font-size: 13px;
  color: var(--text-muted);
}
.error { color: var(--red); }
.timeline-list {
  flex: 1;
  overflow-y: auto;
  padding-left: 8px;
}
.timeline-list::-webkit-scrollbar { width: 3px; }
.timeline-list::-webkit-scrollbar-track { background: transparent; }
.timeline-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }
.timeline-item {
  position: relative;
  padding-bottom: 14px;
  cursor: pointer;
}
.timeline-item::before {
  content: '';
  position: absolute;
  left: 5px;
  top: 12px;
  bottom: -4px;
  width: 1px;
  background: var(--border);
}
.timeline-item:last-child::before { display: none; }
.timeline-dot {
  position: absolute;
  left: 0;
  top: 4px;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}
.timeline-content { margin-left: 20px; }
.timeline-date {
  display: block;
  font-size: 11px;
  color: var(--text-dim);
  margin-bottom: 3px;
}
.timeline-preview {
  margin: 0;
  font-size: 13px;
  color: var(--text-muted);
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.timeline-item:hover .timeline-preview { color: var(--text); }
</style>
