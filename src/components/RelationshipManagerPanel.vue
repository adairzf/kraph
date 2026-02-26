<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { ElNotification } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { useGraphStore } from '../stores/graphStore'
import { getCharacterProfile, getCurrentMemoryLibrary, getMemoriesList } from '../utils/tauriApi'

interface PersonNodeItem {
  id: number
  name: string
  attributes?: string | null
}

interface EventBaseItem {
  id: number
  name: string
  date: Date
  dateKey: string
  days: number
  personIds: number[]
  locationNames: string[]
}

interface EventReminderItem {
  id: number
  name: string
  dateKey: string
  days: number
  people: string[]
  locations: string[]
  tips: string[]
  prep: string[]
}

type ProfileCache = {
  memories: Array<{ id: number; content: string; created_at: string }>
}

const EVENT_WINDOW_DAYS = 30
const NOTIFY_DAYS = 3

const { t } = useI18n()
const graphStore = useGraphStore()

const libraryId = ref('default')
const libraryName = ref('default')
const enabledForLibrary = ref(true)
const remindedOn = ref<Record<string, string>>({})
const eventLoading = ref(false)
const upcomingEvents = ref<EventReminderItem[]>([])

const enabledStorageKey = computed(() => `relationship-manager.enabled.v1.${libraryId.value}`)
const reminderStorageKey = computed(() => `relationship-manager.reminder.v1.${libraryId.value}`)

let eventBuildToken = 0
const profileCache = new Map<number, ProfileCache>()

const people = computed<PersonNodeItem[]>(() => {
  const nodes = graphStore.graphData?.nodes ?? []
  return nodes
    .filter((n) => n.type === 'Person')
    .map((n) => ({
      id: Number(n.id),
      name: n.name,
      attributes: n.attributes,
    }))
    .filter((n) => Number.isInteger(n.id))
    .sort((a, b) => a.name.localeCompare(b.name))
})

const locationNames = computed<string[]>(() => {
  const nodes = graphStore.graphData?.nodes ?? []
  return nodes
    .filter((n) => n.type === 'Location')
    .map((n) => n.name)
    .filter((name) => !!name && name.trim().length > 0)
    .sort((a, b) => b.length - a.length)
})

const personMap = computed(() => {
  const map = new Map<number, PersonNodeItem>()
  for (const person of people.value) {
    map.set(person.id, person)
  }
  return map
})

function parseAttributesToText(attributes?: string | null): string {
  if (!attributes?.trim()) return ''
  try {
    const parsed = JSON.parse(attributes)
    const out: string[] = []
    const walk = (value: unknown) => {
      if (value == null) return
      if (typeof value === 'string') {
        out.push(value)
        return
      }
      if (Array.isArray(value)) {
        value.forEach(walk)
        return
      }
      if (typeof value === 'object') {
        Object.values(value as Record<string, unknown>).forEach(walk)
      }
    }
    walk(parsed)
    return out.join(' | ')
  } catch {
    return attributes
  }
}

function weekdayIndex(ch: string): number | null {
  const map: Record<string, number> = {
    一: 0,
    二: 1,
    三: 2,
    四: 3,
    五: 4,
    六: 5,
    日: 6,
    天: 6,
  }
  return map[ch] ?? null
}

function weekStartMonday(base: Date): Date {
  const d = new Date(base.getFullYear(), base.getMonth(), base.getDate())
  const delta = (d.getDay() + 6) % 7
  d.setDate(d.getDate() - delta)
  d.setHours(0, 0, 0, 0)
  return d
}

function parseChineseNumberToken(token: string): number | null {
  if (/^\d+$/.test(token)) {
    const n = Number(token)
    return Number.isFinite(n) ? n : null
  }

  const normalized = token.replace(/两/g, '二')
  const digitMap: Record<string, number> = {
    零: 0,
    一: 1,
    二: 2,
    三: 3,
    四: 4,
    五: 5,
    六: 6,
    七: 7,
    八: 8,
    九: 9,
  }

  if (normalized === '十') return 10

  if (normalized.includes('十')) {
    const [left, right] = normalized.split('十')
    const tens = left ? digitMap[left] : 1
    const ones = right ? digitMap[right] : 0
    if (tens == null || ones == null) return null
    return tens * 10 + ones
  }

  if ([...normalized].every((ch) => digitMap[ch] != null)) {
    return [...normalized].reduce((acc, ch) => acc * 10 + digitMap[ch], 0)
  }

  return null
}

function extractUpcomingDateFromText(text: string): Date | null {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())

  const absolute = text.match(/\b((?:19|20)\d{2})[-/.年](\d{1,2})[-/.月](\d{1,2})日?\b/)
  if (absolute) {
    const year = Number(absolute[1])
    const month = Number(absolute[2])
    const day = Number(absolute[3])
    if (month >= 1 && month <= 12 && day >= 1 && day <= 31) {
      const d = new Date(year, month - 1, day)
      d.setHours(0, 0, 0, 0)
      return d
    }
  }

  const dayOffsetMatch = text.match(/([0-9零一二两三四五六七八九十]{1,3})\s*[天日](?:后|之后)/)
  if (dayOffsetMatch) {
    const offset = parseChineseNumberToken(dayOffsetMatch[1])
    if (offset != null && offset >= 0 && offset <= 365) {
      const d = new Date(today)
      d.setDate(d.getDate() + offset)
      return d
    }
  }

  if (text.includes('后天')) {
    const d = new Date(today)
    d.setDate(d.getDate() + 2)
    return d
  }
  if (text.includes('明天')) {
    const d = new Date(today)
    d.setDate(d.getDate() + 1)
    return d
  }
  if (text.includes('今天')) {
    return today
  }

  const weekMatch = text.match(/(下周|下星期|本周|这周|周)(末|[一二三四五六日天])/)
  if (weekMatch) {
    const scope = weekMatch[1]
    const dayToken = weekMatch[2]
    const base = weekStartMonday(today)
    const weekOffset = scope === '下周' || scope === '下星期' ? 1 : 0
    let targetWeekday = dayToken === '末' ? 5 : weekdayIndex(dayToken)
    if (targetWeekday == null) targetWeekday = 0
    const d = new Date(base)
    d.setDate(d.getDate() + weekOffset * 7 + targetWeekday)
    if (scope === '周' && d < today) {
      d.setDate(d.getDate() + 7)
    }
    return d
  }

  const md = text.match(/\b(\d{1,2})[-/.月](\d{1,2})日?\b/)
  if (md) {
    const month = Number(md[1])
    const day = Number(md[2])
    if (month >= 1 && month <= 12 && day >= 1 && day <= 31) {
      const year = today.getFullYear()
      const d = new Date(year, month - 1, day)
      d.setHours(0, 0, 0, 0)
      if (d < today) {
        d.setFullYear(year + 1)
      }
      return d
    }
  }

  return null
}

function daysUntil(date: Date): number {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  return Math.max(0, Math.round((date.getTime() - today.getTime()) / (24 * 60 * 60 * 1000)))
}

function dateKey(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

function todayKey(): string {
  return dateKey(new Date())
}

function loadEnabledState() {
  const raw = localStorage.getItem(enabledStorageKey.value)
  enabledForLibrary.value = raw == null ? true : raw === '1'
}

function persistEnabledState() {
  localStorage.setItem(enabledStorageKey.value, enabledForLibrary.value ? '1' : '0')
}

function loadReminderLog() {
  const raw = localStorage.getItem(reminderStorageKey.value)
  if (!raw) {
    remindedOn.value = {}
    return
  }
  try {
    const parsed = JSON.parse(raw)
    remindedOn.value = parsed && typeof parsed === 'object' ? parsed : {}
  } catch {
    remindedOn.value = {}
  }
}

function persistReminderLog() {
  localStorage.setItem(reminderStorageKey.value, JSON.stringify(remindedOn.value))
}

async function syncCurrentLibrary() {
  try {
    const library = await getCurrentMemoryLibrary()
    libraryId.value = library.id
    libraryName.value = library.name || library.id
  } catch {
    libraryId.value = 'default'
    libraryName.value = 'default'
  }
}

async function applyCurrentLibraryContext() {
  await syncCurrentLibrary()
  loadEnabledState()
  loadReminderLog()
  await refreshUpcomingEvents()
  maybeNotifyEvents()
}

function eventBaseItemsFromGraph(): EventBaseItem[] {
  const graph = graphStore.graphData
  if (!graph) return []

  const nodeById = new Map<string, { id: string; name: string; type: string; attributes: string | null }>()
  for (const node of graph.nodes) {
    nodeById.set(node.id, node)
  }

  const output: EventBaseItem[] = []
  for (const eventNode of graph.nodes.filter((n) => n.type === 'Event')) {
    const personIds = new Set<number>()
    const locationNames = new Set<string>()
    const textParts = [eventNode.name, parseAttributesToText(eventNode.attributes)]

    for (const link of graph.links) {
      let otherId: string | null = null
      if (link.source === eventNode.id) otherId = link.target
      if (link.target === eventNode.id) otherId = link.source
      if (!otherId) continue
      const other = nodeById.get(otherId)
      if (!other) continue

      if (other.type === 'Person') {
        const id = Number(other.id)
        if (Number.isInteger(id)) personIds.add(id)
      } else if (other.type === 'Location') {
        locationNames.add(other.name)
      } else if (other.type === 'Time') {
        textParts.push(other.name)
        textParts.push(parseAttributesToText(other.attributes))
      }
    }

    const date = extractUpcomingDateFromText(textParts.filter(Boolean).join(' | '))
    if (!date) continue
    const days = daysUntil(date)
    if (days > EVENT_WINDOW_DAYS) continue

    output.push({
      id: Number(eventNode.id),
      name: eventNode.name,
      date,
      dateKey: dateKey(date),
      days,
      personIds: [...personIds],
      locationNames: [...locationNames],
    })
  }

  return output.sort((a, b) => a.days - b.days)
}

async function eventBaseItemsFromMemoryFallback(): Promise<EventBaseItem[]> {
  const memories = await getMemoriesList()
  const peopleByName = [...people.value].sort((a, b) => b.name.length - a.name.length)
  const places = locationNames.value
  const output: EventBaseItem[] = []
  const seen = new Set<string>()

  for (const mem of memories.slice(0, 240)) {
    const candidates = splitSentences(mem.content)
    const segments = candidates.length ? candidates : [mem.content.trim()]

    for (let idx = 0; idx < segments.length; idx++) {
      const segment = segments[idx].trim()
      if (!segment) continue

      const date = extractUpcomingDateFromText(segment)
      if (!date) continue
      const days = daysUntil(date)
      if (days > EVENT_WINDOW_DAYS) continue

      const personIds = new Set<number>()
      for (const p of peopleByName) {
        if (segment.includes(p.name)) {
          personIds.add(p.id)
        }
      }

      const matchedLocations = new Set<string>()
      for (const place of places) {
        if (segment.includes(place)) {
          matchedLocations.add(place)
        }
      }

      const key = `${dateKey(date)}|${segment}`
      if (seen.has(key)) continue
      seen.add(key)

      const eventName = segment.length > 42 ? `${segment.slice(0, 42)}…` : segment
      output.push({
        id: 1_000_000_000 + mem.id * 10 + idx,
        name: eventName,
        date,
        dateKey: dateKey(date),
        days,
        personIds: [...personIds],
        locationNames: [...matchedLocations],
      })
    }
  }

  return output.sort((a, b) => a.days - b.days)
}

async function getProfileCache(personId: number): Promise<ProfileCache> {
  const cached = profileCache.get(personId)
  if (cached) return cached
  const profile = await getCharacterProfile(personId)
  const info: ProfileCache = {
    memories: profile.memories.slice(0, 12),
  }
  profileCache.set(personId, info)
  return info
}

function splitSentences(text: string): string[] {
  return text
    .split(/[。！？；;!?\n]/)
    .map((x) => x.trim())
    .filter((x) => x.length >= 6)
}

function extractPersonHints(personName: string, memories: Array<{ content: string }>): string[] {
  const keywords = [
    '喜欢', '拍照', '摄影', '不舒服', '腿', '膝', '腰', '疼', '过敏', '忌口', '不能吃',
    '注意', '不便', '生病', '药', '偏好',
  ]
  const set = new Set<string>()
  for (const mem of memories) {
    for (const sentence of splitSentences(mem.content)) {
      const containsName = sentence.includes(personName)
      const containsKeyword = keywords.some((k) => sentence.includes(k))
      if (!containsName && !containsKeyword) continue
      set.add(sentence)
      if (set.size >= 4) return [...set]
    }
  }
  return [...set]
}

function buildPrepList(hints: string[], hasLocation: boolean, days: number): string[] {
  const prep = new Set<string>()
  const fullText = hints.join(' | ')

  if (/拍照|摄影|照片/.test(fullText)) {
    prep.add(t('relationshipManager.events.prep.camera'))
  }
  if (/腿|膝|腰|不舒服|疼|受伤|扭伤|生病/.test(fullText)) {
    prep.add(t('relationshipManager.events.prep.comfort'))
  }
  if (/过敏|忌口|不能吃|胃/.test(fullText)) {
    prep.add(t('relationshipManager.events.prep.food'))
  }
  if (hasLocation) {
    prep.add(t('relationshipManager.events.prep.route'))
  }
  if (days <= 1) {
    prep.add(t('relationshipManager.events.prep.confirm'))
  }
  if (!prep.size) {
    prep.add(t('relationshipManager.events.prep.default'))
  }

  return [...prep]
}

async function refreshUpcomingEvents() {
  if (!enabledForLibrary.value) {
    upcomingEvents.value = []
    eventLoading.value = false
    return
  }

  const graph = graphStore.graphData
  if (!graph) {
    upcomingEvents.value = []
    return
  }

  eventLoading.value = true
  const token = ++eventBuildToken

  try {
    let base = eventBaseItemsFromGraph()
    if (!base.length) {
      base = await eventBaseItemsFromMemoryFallback()
    }
    base = base.slice(0, 12)
    const result: EventReminderItem[] = []

    for (const item of base) {
      const peopleNames = item.personIds
        .map((id) => personMap.value.get(id)?.name)
        .filter((x): x is string => !!x)

      const hints: string[] = []
      for (const personId of item.personIds.slice(0, 3)) {
        try {
          const cached = await getProfileCache(personId)
          const name = personMap.value.get(personId)?.name ?? ''
          const extracted = extractPersonHints(name, cached.memories)
          for (const line of extracted) {
            if (!hints.includes(line)) hints.push(line)
            if (hints.length >= 4) break
          }
          if (hints.length >= 4) break
        } catch {
          // Ignore per-person recall error to avoid blocking the entire reminder list.
        }
      }

      result.push({
        id: item.id,
        name: item.name,
        dateKey: item.dateKey,
        days: item.days,
        people: peopleNames,
        locations: item.locationNames,
        tips: hints,
        prep: buildPrepList(hints, item.locationNames.length > 0, item.days),
      })
    }

    if (token === eventBuildToken) {
      upcomingEvents.value = result
    }
  } finally {
    if (token === eventBuildToken) {
      eventLoading.value = false
    }
  }
}

function maybeNotifyEvents() {
  if (!enabledForLibrary.value) return

  const today = todayKey()
  for (const item of upcomingEvents.value) {
    if (item.days > NOTIFY_DAYS) continue
    const logKey = `event:${item.id}:${item.dateKey}`
    if (remindedOn.value[logKey] === today) continue

    const firstPrep = item.prep[0] ?? t('relationshipManager.events.prep.default')
    ElNotification({
      title: t('relationshipManager.events.title'),
      message: t('relationshipManager.events.notify', {
        event: item.name,
        days: item.days,
        date: item.dateKey,
        prep: firstPrep,
      }),
      duration: 5000,
      position: 'bottom-right',
    })

    remindedOn.value = {
      ...remindedOn.value,
      [logKey]: today,
    }
  }
}

watch(enabledStorageKey, () => {
  loadEnabledState()
})

watch(reminderStorageKey, () => {
  loadReminderLog()
  maybeNotifyEvents()
})

watch(enabledForLibrary, async () => {
  persistEnabledState()
  profileCache.clear()
  await refreshUpcomingEvents()
  maybeNotifyEvents()
})

watch(remindedOn, persistReminderLog, { deep: true })
watch(upcomingEvents, maybeNotifyEvents)

watch(
  () => graphStore.graphData,
  async () => {
    profileCache.clear()
    await applyCurrentLibraryContext()
  },
  { deep: false },
)

function onMemoryLibraryChanged() {
  profileCache.clear()
  void applyCurrentLibraryContext()
}

onMounted(async () => {
  if (!graphStore.graphData) {
    await graphStore.fetchGraph()
  }
  window.addEventListener('memory-library-changed', onMemoryLibraryChanged)
  await applyCurrentLibraryContext()
})

onUnmounted(() => {
  window.removeEventListener('memory-library-changed', onMemoryLibraryChanged)
})
</script>

<template>
  <div class="relationship-panel">
    <h2 class="panel-title">{{ t('relationshipManager.title') }}</h2>
    <p class="hint">{{ t('relationshipManager.hint') }}</p>

    <section class="section">
      <div class="switch-row">
        <div>
          <h3>{{ t('relationshipManager.scope.title') }}</h3>
          <p class="subtle">
            {{ t('relationshipManager.scope.libraryStatus', { name: libraryName }) }}
          </p>
        </div>
        <el-switch v-model="enabledForLibrary" />
      </div>
      <p class="subtle">
        {{
          enabledForLibrary
            ? t('relationshipManager.scope.enabledHint')
            : t('relationshipManager.scope.disabledHint')
        }}
      </p>
    </section>

    <section class="section">
      <h3>{{ t('relationshipManager.events.title') }}</h3>
      <p class="subtle">{{ t('relationshipManager.events.hint') }}</p>
      <p class="subtle">{{ t('relationshipManager.events.trigger', { days: NOTIFY_DAYS }) }}</p>

      <p v-if="!enabledForLibrary" class="empty">{{ t('relationshipManager.events.disabled') }}</p>
      <p v-else-if="eventLoading" class="empty">{{ t('relationshipManager.events.loading') }}</p>
      <ul v-else-if="upcomingEvents.length" class="event-list">
        <li v-for="item in upcomingEvents" :key="`event-${item.id}`" class="event-item">
          <div class="event-header">
            <strong>{{ item.name }}</strong>
            <span>{{ item.dateKey }} · {{ t('relationshipManager.events.daysLeft', { days: item.days }) }}</span>
          </div>

          <p v-if="item.people.length" class="event-meta">
            {{ t('relationshipManager.events.withPeople') }} {{ item.people.join('、') }}
          </p>
          <p v-if="item.locations.length" class="event-meta">
            {{ t('relationshipManager.events.atLocations') }} {{ item.locations.join('、') }}
          </p>

          <p class="event-subtitle">{{ t('relationshipManager.events.prepTitle') }}</p>
          <ul class="mini-list">
            <li v-for="(line, idx) in item.prep" :key="`prep-${item.id}-${idx}`">{{ line }}</li>
          </ul>

          <template v-if="item.tips.length">
            <p class="event-subtitle">{{ t('relationshipManager.events.contextTitle') }}</p>
            <ul class="mini-list">
              <li v-for="(line, idx) in item.tips" :key="`tip-${item.id}-${idx}`">{{ line }}</li>
            </ul>
          </template>
        </li>
      </ul>
      <p v-else class="empty">{{ t('relationshipManager.events.empty') }}</p>
    </section>
  </div>
</template>

<style scoped>
.relationship-panel { padding: 0; }
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
.subtle {
  margin: 4px 0 0 0;
  color: var(--text-dim);
  font-size: 12px;
}
.section {
  margin-top: 10px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.switch-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.section h3 {
  margin: 0;
  font-size: 13px;
  color: var(--text);
}
.event-list {
  list-style: none;
  margin: 8px 0 0 0;
  padding: 0;
  max-height: 320px;
  overflow: auto;
}
.event-item {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 8px;
  margin-bottom: 8px;
  background: var(--bg4);
}
.event-header {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 12px;
  color: var(--text);
}
.event-meta {
  margin: 3px 0 0 0;
  font-size: 11px;
  color: var(--text-dim);
}
.event-subtitle {
  margin: 6px 0 2px 0;
  font-size: 11px;
  color: var(--text-dim);
}
.mini-list {
  margin: 0;
  padding-left: 16px;
}
.mini-list li {
  font-size: 12px;
  color: var(--text-muted);
  margin: 2px 0;
}
.empty {
  margin: 8px 0 0 0;
  color: var(--text-dim);
  font-size: 12px;
}

@media (max-width: 880px) {
  .event-header,
  .switch-row {
    flex-direction: column;
  }
}
</style>
