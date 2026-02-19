<script setup lang="ts">
import { ref, watch } from 'vue'
import { getCharacterProfile, queryEntity } from '../utils/tauriApi'
import type { Entity } from '../types/entity'
import type { Memory } from '../types/memory'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{ entityId: number | null; entityName: string | null }>()

interface RelationItem {
  from_entity_id: number
  from_name: string
  to_entity_id: number
  to_name: string
  relation_type: string
  strength: number
}

const profile = ref<{
  entity: Entity
  memories: Memory[]
  relations: RelationItem[]
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
      error.value = t('characterCard.notFound')
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

// 解析 attributes 并提取所有标签
function parseAttributes(attributes: string | null | undefined): string[] {
  if (!attributes) return []
  
  try {
    const attrs = typeof attributes === 'string' ? JSON.parse(attributes) : attributes
    const tags: string[] = []
    
    // 遍历所有属性值，将逗号分隔的值拆分成单独的标签
    Object.values(attrs).forEach((value) => {
      if (typeof value === 'string') {
        // 按逗号分隔并去除空格
        const splitTags = value.split(',').map(tag => tag.trim()).filter(tag => tag)
        tags.push(...splitTags)
      }
    })
    
    return tags
  } catch (e) {
    // 如果解析失败，返回空数组
    return []
  }
}
</script>

<template>
  <div class="character-card">
    <h3 class="card-title">{{ t('characterCard.title') }}</h3>
    <p v-if="loading" class="loading">{{ t('characterCard.loading') }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <template v-else-if="profile">
      <div class="entity-header">
        <span class="entity-type">{{ profile.entity.type }}</span>
        <strong class="entity-name">{{ profile.entity.name }}</strong>
        <div v-if="parseAttributes(profile.entity.attributes).length" class="entity-attrs">
          <span 
            v-for="(tag, index) in parseAttributes(profile.entity.attributes)" 
            :key="index" 
            class="attr-tag"
          >
            {{ tag }}
          </span>
        </div>
      </div>
      <div v-if="profile.relations.length" class="section">
        <h4>{{ t('characterCard.relations') }}</h4>
        <ul class="relations">
          <li v-for="(r, i) in profile.relations" :key="i" class="relation-item">
            <span
              class="rel-entity"
              :class="{ 'rel-self': r.from_entity_id === profile.entity.id }"
            >{{ r.from_name }}</span>
            <span class="rel-type">{{ r.relation_type }}</span>
            <span
              class="rel-entity"
              :class="{ 'rel-self': r.to_entity_id === profile.entity.id }"
            >{{ r.to_name }}</span>
          </li>
        </ul>
      </div>
      <div class="section">
        <h4>{{ t('characterCard.relatedMemories', { count: profile.memories.length }) }}</h4>
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
    <p v-else class="hint">{{ t('characterCard.hint') }}</p>
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
  margin: 0.5rem 0 0 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.attr-tag {
  display: inline-block;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  background: #f0f0f0;
  color: #555;
  font-size: 0.75rem;
  line-height: 1.2;
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
.relation-item {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.2rem 0;
  border-bottom: 1px solid #f0f0f0;
  flex-wrap: wrap;
  line-height: 1.5;
}
.relation-item:last-child {
  border-bottom: none;
}
.rel-entity {
  color: #333;
  font-weight: 500;
}
.rel-entity.rel-self {
  color: #24c8db;
  font-weight: 600;
}
.rel-type {
  color: #888;
  font-size: 0.8rem;
  padding: 0.05rem 0.35rem;
  background: #f5f5f5;
  border-radius: 3px;
  white-space: nowrap;
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
