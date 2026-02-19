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

// Parse the attributes JSON and flatten all values into individual tags.
function parseAttributes(attributes: string | null | undefined): string[] {
  if (!attributes) return []

  try {
    const attrs = typeof attributes === 'string' ? JSON.parse(attributes) : attributes
    const tags: string[] = []

    // Split comma-separated attribute values into individual tag strings
    Object.values(attrs).forEach((value) => {
      if (typeof value === 'string') {
        const splitTags = value.split(',').map(tag => tag.trim()).filter(tag => tag)
        tags.push(...splitTags)
      }
    })

    return tags
  } catch {
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
  font-size: 13px;
}
.card-title {
  margin: 0 0 10px 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
}
.loading,
.hint {
  margin: 4px 0;
  color: var(--text-dim);
  font-size: 13px;
}
.error {
  margin: 4px 0;
  color: var(--red);
  font-size: 13px;
}
.entity-header {
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.entity-type {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 100px;
  background: rgba(124, 92, 252, 0.12);
  border: 1px solid rgba(124, 92, 252, 0.25);
  color: #a78bfa;
  font-size: 11px;
  font-weight: 500;
}
.entity-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.entity-attrs {
  width: 100%;
  margin-top: 6px;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.attr-tag {
  display: inline-block;
  padding: 2px 7px;
  border-radius: 4px;
  background: var(--bg4);
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 11px;
}
.section { margin-top: 12px; }
.section h4 {
  margin: 0 0 6px 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--text-dim);
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
  gap: 5px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
  flex-wrap: wrap;
  line-height: 1.5;
}
.relation-item:last-child { border-bottom: none; }
.rel-entity {
  color: var(--text);
  font-weight: 500;
  font-size: 13px;
}
.rel-entity.rel-self {
  color: #a78bfa;
  font-weight: 600;
}
.rel-type {
  color: var(--text-dim);
  font-size: 12px;
  padding: 1px 6px;
  background: var(--bg4);
  border: 1px solid var(--border);
  border-radius: 4px;
  white-space: nowrap;
}
.memory-item {
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
  line-height: 1.45;
  color: var(--text-muted);
  font-size: 13px;
}
.memory-item::before {
  content: '· ';
  color: var(--text-dim);
}
.memory-item:last-child { border-bottom: none; }
</style>
