<script setup lang="ts">
import { ref, watch } from 'vue'
import { getCharacterProfile, queryEntity } from '../utils/tauriApi'
import type { Entity } from '../types/entity'
import type { Memory } from '../types/memory'

const props = defineProps<{ entityId: number | null; entityName: string | null }>()

const profile = ref<{
  entity: Entity
  memories: Memory[]
  relations: { from_entity_id: number; to_entity_id: number; relation_type: string }[]
} | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)

async function loadByEntityId(id: number) {
  loading.value = true
  error.value = null
  try {
    profile.value = await getCharacterProfile(id)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
    profile.value = null
  } finally {
    loading.value = false
  }
}

async function loadByEntityName(name: string) {
  loading.value = true
  error.value = null
  try {
    const entity = await queryEntity(name)
    if (entity) {
      profile.value = await getCharacterProfile(entity.id)
    } else {
      profile.value = null
      error.value = '未找到该实体'
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
    profile.value = null
  } finally {
    loading.value = false
  }
}

watch(
  () => props.entityId,
  (id) => {
    if (id != null) loadByEntityId(id)
    else if (!props.entityName) {
      profile.value = null
      error.value = null
    }
  },
  { immediate: true }
)

watch(
  () => props.entityName,
  (name) => {
    if (name?.trim()) loadByEntityName(name.trim())
    else if (props.entityId == null) {
      profile.value = null
      error.value = null
    }
  }
)

function preview(content: string) {
  return content.length > 80 ? content.slice(0, 80) + '…' : content
}
</script>

<template>
  <div class="character-card">
    <h3 class="card-title">角色档案</h3>
    <p v-if="loading" class="loading">加载中…</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <template v-else-if="profile">
      <div class="entity-header">
        <span class="entity-type">{{ profile.entity.type }}</span>
        <strong class="entity-name">{{ profile.entity.name }}</strong>
        <p v-if="profile.entity.attributes" class="entity-attrs">
          {{ profile.entity.attributes }}
        </p>
      </div>
      <div v-if="profile.relations.length" class="section">
        <h4>关系</h4>
        <ul class="relations">
          <li
            v-for="(r, i) in profile.relations"
            :key="i"
          >
            {{ r.relation_type }}
          </li>
        </ul>
      </div>
      <div class="section">
        <h4>相关记忆 ({{ profile.memories.length }})</h4>
        <ul class="memories">
          <li
            v-for="m in profile.memories.slice(0, 5)"
            :key="m.id"
            class="memory-item"
          >
            {{ preview(m.content) }}
          </li>
        </ul>
      </div>
    </template>
    <p v-else class="hint">在右侧图谱点击节点，或在上方搜索实体名称查看档案。</p>
  </div>
</template>

<style scoped>
.character-card {
  padding: 0.75rem 0;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
  font-size: 0.875rem;
}
.card-title {
  margin: 0 0 0.5rem 0;
  font-size: 0.9375rem;
  font-weight: 600;
}
.loading,
.error,
.hint {
  margin: 0.25rem 0;
  color: #666;
}
.error {
  color: var(--color-error, #c00);
}
.entity-header {
  margin-bottom: 0.75rem;
}
.entity-type {
  display: inline-block;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  background: #e8f4f8;
  color: #24c8db;
  font-size: 0.75rem;
  margin-right: 0.5rem;
}
.entity-name {
  font-size: 1rem;
}
.entity-attrs {
  margin: 0.25rem 0 0 0;
  color: #666;
}
.section {
  margin-top: 0.75rem;
}
.section h4 {
  margin: 0 0 0.35rem 0;
  font-size: 0.8125rem;
  color: #666;
}
.relations,
.memories {
  list-style: none;
  margin: 0;
  padding: 0;
}
.memory-item {
  padding: 0.25rem 0;
  border-bottom: 1px solid #eee;
  line-height: 1.4;
}
.memory-item:last-child {
  border-bottom: none;
}
</style>
