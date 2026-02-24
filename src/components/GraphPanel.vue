<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useGraphStore } from '../stores/graphStore'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { GraphChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
} from 'echarts/components'
import { useI18n } from 'vue-i18n'
import type { EChartsType } from 'echarts/core'

use([
  CanvasRenderer,
  GraphChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
])

const graphStore = useGraphStore()
const { t } = useI18n()
const themeRefreshTick = ref(0)
const chartRef = ref<InstanceType<typeof VChart> | null>(null)
const selectedTypes = ref<string[]>([])
const typesInitialized = ref(false)
const pendingCenterNodeId = ref<string | null>(null)
const backgroundClickDisposer = ref<(() => void) | null>(null)
const backgroundClickBoundChart = ref<EChartsType | null>(null)
const GRAPH_VISIBLE_TYPES_KEY = 'graph-visible-types'

const nodeColors: Record<string, string> = {
  Person: '#7c5cfc',
  Location: '#34d399',
  Event: '#f472b6',
  Time: '#fb923c',
  Organization: '#3b82f6',
}

type GraphLinkItem = {
  source: string
  target: string
  relation: string
  strength: number
}

const availableTypes = computed(() => {
  const nodes = graphStore.graphData?.nodes ?? []
  return [...new Set(nodes.map((n) => n.type))].sort((a, b) => a.localeCompare(b))
})

const visibleTypeSet = computed(() => new Set(selectedTypes.value))
const allTypesSelected = computed(() =>
  availableTypes.value.length > 0 && selectedTypes.value.length === availableTypes.value.length,
)

function loadVisibleTypesFromStorage(): string[] {
  const raw = localStorage.getItem(GRAPH_VISIBLE_TYPES_KEY)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return [...new Set(parsed.map((x) => String(x)))]
  } catch {
    return []
  }
}

function saveVisibleTypesToStorage() {
  localStorage.setItem(GRAPH_VISIBLE_TYPES_KEY, JSON.stringify(selectedTypes.value))
}

function typeDisplayName(type: string): string {
  const key = `graphPanel.types.${type}`
  const translated = t(key)
  return translated === key ? type : translated
}

function toggleType(type: string) {
  const set = new Set(selectedTypes.value)
  if (set.has(type)) {
    set.delete(type)
  } else {
    set.add(type)
  }
  selectedTypes.value = availableTypes.value.filter((x) => set.has(x))
}

function toggleAllTypes() {
  if (allTypesSelected.value) {
    selectedTypes.value = []
    return
  }
  selectedTypes.value = [...availableTypes.value]
}

function currentTheme() {
  return document.documentElement.classList.contains('light') ? 'light' : 'dark'
}

function onThemeChanged() {
  themeRefreshTick.value += 1
}

function linkPairKey(source: string, target: string): string {
  return source < target ? `${source}::${target}` : `${target}::${source}`
}

function clampCurveness(value: number): number {
  return Math.max(-0.55, Math.min(0.55, value))
}

function buildCurvedLinks(
  rawLinks: GraphLinkItem[],
  isLight: boolean,
  edgeLineColor: string,
) {
  const pairGroups = new Map<string, number[]>()
  rawLinks.forEach((link, index) => {
    const key = linkPairKey(link.source, link.target)
    const group = pairGroups.get(key)
    if (group) {
      group.push(index)
    } else {
      pairGroups.set(key, [index])
    }
  })

  const curvenessByIndex = new Array(rawLinks.length).fill(0)

  for (const indices of pairGroups.values()) {
    if (indices.length <= 1) continue

    const directionGroups = new Map<string, number[]>()
    for (const index of indices) {
      const link = rawLinks[index]
      const dirKey = `${link.source}->${link.target}`
      const group = directionGroups.get(dirKey)
      if (group) {
        group.push(index)
      } else {
        directionGroups.set(dirKey, [index])
      }
    }

    const directionKeys = [...directionGroups.keys()].sort((a, b) => a.localeCompare(b))

    if (directionKeys.length === 2) {
      const [dirA, dirB] = directionKeys
      const groupA = (directionGroups.get(dirA) ?? []).slice()
      const groupB = (directionGroups.get(dirB) ?? []).slice()
      groupA.sort((ia, ib) => rawLinks[ia].relation.localeCompare(rawLinks[ib].relation))
      groupB.sort((ia, ib) => rawLinks[ia].relation.localeCompare(rawLinks[ib].relation))

      groupA.forEach((index, order) => {
        curvenessByIndex[index] = clampCurveness(0.2 + order * 0.1)
      })
      groupB.forEach((index, order) => {
        // For reversed direction, using the same sign keeps the curve on the opposite side
        // because ECharts computes normal vector from source->target.
        curvenessByIndex[index] = clampCurveness(0.2 + order * 0.1)
      })
      continue
    }

    const ordered = indices
      .slice()
      .sort((ia, ib) => rawLinks[ia].relation.localeCompare(rawLinks[ib].relation))
    const mid = (ordered.length - 1) / 2
    ordered.forEach((index, order) => {
      curvenessByIndex[index] = clampCurveness((order - mid) * 0.12)
    })
  }

  return rawLinks.map((l, index) => ({
    source: l.source,
    target: l.target,
    value: l.relation,
    lineStyle: {
      width: Math.max(isLight ? 1.8 : 1, Math.min(l.strength, isLight ? 3.4 : 3)),
      color: edgeLineColor,
      curveness: curvenessByIndex[index],
    },
  }))
}

const visibleGraphData = computed(() => {
  const g = graphStore.graphData
  if (!g) return null

  const nodes = g.nodes.filter((n) => visibleTypeSet.value.has(n.type))
  const nodeIdSet = new Set(nodes.map((n) => n.id))
  const links = g.links.filter((l) => nodeIdSet.has(l.source) && nodeIdSet.has(l.target))

  return { nodes, links }
})

const matchedNodeId = computed(() => {
  const keyword = (graphStore.searchEntityName ?? '').trim().toLowerCase()
  if (!keyword) return null
  const nodes = visibleGraphData.value?.nodes ?? []
  const exact = nodes.find((n) => n.name.trim().toLowerCase() === keyword)
  if (exact) return exact.id
  const fuzzy = nodes.find((n) => n.name.trim().toLowerCase().includes(keyword))
  return fuzzy?.id ?? null
})
const selectedNodeId = computed(() =>
  graphStore.selectedEntityId != null ? String(graphStore.selectedEntityId) : null,
)

watch(
  availableTypes,
  (types) => {
    if (!types.length) {
      selectedTypes.value = []
      return
    }
    if (!typesInitialized.value) {
      const stored = loadVisibleTypesFromStorage()
      const validStored = types.filter((type) => stored.includes(type))
      selectedTypes.value = validStored.length ? validStored : [...types]
      typesInitialized.value = true
      return
    }
    selectedTypes.value = selectedTypes.value.filter((type) => types.includes(type))
  },
  { immediate: true },
)

watch(selectedTypes, saveVisibleTypesToStorage, { deep: true })

watch(
  [visibleGraphData, selectedNodeId],
  ([g, selectedId]) => {
    if (!selectedId || !g) return
    const exists = g.nodes.some((n) => n.id === selectedId)
    if (!exists) {
      graphStore.setSelectedEntity(null)
    }
  },
  { deep: false },
)

watch(
  matchedNodeId,
  (nodeId) => {
    pendingCenterNodeId.value = nodeId
    if (!nodeId) return
    nextTick(() => {
      centerNodeById(nodeId)
    })
  },
  { immediate: true },
)

watch(
  () => graphStore.searchEntityName,
  (name) => {
    const keyword = (name ?? '').trim().toLowerCase()
    if (keyword) {
      const hit = (graphStore.graphData?.nodes ?? []).find((n) => n.name.trim().toLowerCase() === keyword)
        ?? (graphStore.graphData?.nodes ?? []).find((n) => n.name.trim().toLowerCase().includes(keyword))
      if (hit && !visibleTypeSet.value.has(hit.type)) {
        selectedTypes.value = [...new Set([...selectedTypes.value, hit.type])]
      }
    }
    themeRefreshTick.value += 1
  },
)

function getEchartsInstance(): EChartsType | null {
  const chart = chartRef.value as unknown as { chart?: EChartsType } | null
  return chart?.chart ?? null
}

function getNodeDataIndexById(nodeId: string): number {
  const ec = getEchartsInstance()
  if (!ec) return -1
  const ecModel = (ec as unknown as { getModel: () => { getSeriesByIndex: (i: number) => { getData: () => {
    count: () => number
    getId: (i: number) => string
  } } } }).getModel()
  const series = ecModel.getSeriesByIndex(0)
  const data = series?.getData()
  if (!data) return -1
  const count = data.count()
  for (let i = 0; i < count; i++) {
    if (String(data.getId(i)) === nodeId) {
      return i
    }
  }
  return -1
}

function bindBackgroundClickListener() {
  const ec = getEchartsInstance()
  if (!ec || backgroundClickBoundChart.value === ec) return

  if (backgroundClickDisposer.value) {
    backgroundClickDisposer.value()
    backgroundClickDisposer.value = null
  }

  const zr = ec.getZr()
  const onZrClick = (event: { target?: unknown }) => {
    if (event.target) return
    if (graphStore.selectedEntityId != null) {
      graphStore.setSelectedEntity(null)
    }
  }

  zr.on('click', onZrClick)
  backgroundClickBoundChart.value = ec
  backgroundClickDisposer.value = () => {
    zr.off('click', onZrClick)
    backgroundClickBoundChart.value = null
  }
}

function centerNodeById(nodeId: string): boolean {
  const ec = getEchartsInstance()
  if (!ec) {
    return false
  }
  const dataIndex = getNodeDataIndexById(nodeId)
  if (dataIndex < 0) return false

  const ecModel = (ec as unknown as { getModel: () => { getSeriesByIndex: (i: number) => {
    getData: () => {
      getItemLayout: (i: number) => unknown
    }
  } } }).getModel()
  const series = ecModel.getSeriesByIndex(0)
  const data = series?.getData()
  if (!data) return false

  const layout = data.getItemLayout(dataIndex) as { x?: number; y?: number } | number[] | null
  const point = Array.isArray(layout)
    ? [Number(layout[0]), Number(layout[1])]
    : [Number(layout?.x), Number(layout?.y)]
  if (!Number.isFinite(point[0]) || !Number.isFinite(point[1])) {
    return false
  }

  const pixel = ec.convertToPixel({ seriesIndex: 0 }, point) as number[] | null
  if (!pixel || !Number.isFinite(pixel[0]) || !Number.isFinite(pixel[1])) {
    return false
  }

  const centerX = ec.getWidth() / 2
  const centerY = ec.getHeight() / 2
  const dx = centerX - pixel[0]
  const dy = centerY - pixel[1]

  if (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5) {
    ec.dispatchAction({
      type: 'graphRoam',
      seriesIndex: 0,
      dx,
      dy,
    })
  }

  return true
}

function onChartFinished() {
  bindBackgroundClickListener()

  if (pendingCenterNodeId.value && centerNodeById(pendingCenterNodeId.value)) {
    pendingCenterNodeId.value = null
  }
}

const chartOption = computed(() => {
  themeRefreshTick.value
  const isLight = currentTheme() === 'light'
  const emptyTitleColor = isLight ? '#64748b' : '#4a4a5e'
  const nodeLabelColor = isLight ? '#334155' : '#e2e2ee'
  const edgeLabelColor = isLight ? '#64748b' : '#7a7a8e'
  const edgeLineColor = isLight ? 'rgba(51, 65, 85, 0.42)' : 'rgba(255,255,255,0.12)'
  const edgeEmphasisColor = isLight ? 'rgba(30, 41, 59, 0.7)' : 'rgba(255,255,255,0.45)'

  const rawGraph = graphStore.graphData
  const g = visibleGraphData.value
  if (!rawGraph || !g || (!rawGraph.nodes.length && !rawGraph.links.length)) {
    return {
      backgroundColor: 'transparent',
      title: {
        text: t('graphPanel.empty'),
        left: 'center',
        top: 'middle',
        textStyle: { color: emptyTitleColor, fontSize: 14, fontWeight: 'normal' },
      },
    }
  }
  if (!g.nodes.length && rawGraph.nodes.length) {
    return {
      backgroundColor: 'transparent',
      title: {
        text: t('graphPanel.emptyByFilter'),
        left: 'center',
        top: 'middle',
        textStyle: { color: emptyTitleColor, fontSize: 14, fontWeight: 'normal' },
      },
    }
  }
  const nodes = g.nodes.map((n: { id: string; name: string; type: string }) => {
    return {
      id: n.id,
      name: n.name,
      symbolSize: 34,
      itemStyle: {
        color: nodeColors[n.type] ?? '#22d3ee',
        borderColor: 'rgba(255,255,255,0.12)',
        borderWidth: 1,
        shadowBlur: 6,
        shadowColor: (nodeColors[n.type] ?? '#22d3ee') + '50',
      },
      label: {
        show: true,
        fontSize: 12,
        color: nodeLabelColor,
        fontWeight: 500,
        fontFamily: 'Inter, -apple-system, sans-serif',
      },
      z: 1,
    }
  })
  const links = buildCurvedLinks(g.links as GraphLinkItem[], isLight, edgeLineColor)
  return {
    backgroundColor: 'transparent',
    tooltip: {
      backgroundColor: '#16161f',
      borderColor: 'rgba(255,255,255,0.08)',
      textStyle: { color: '#e2e2ee', fontSize: 12 },
      formatter: (params: unknown) => {
        const p = params as { dataType?: string; data?: { name?: string; value?: string } }
        return p.dataType === 'edge' ? p.data?.value ?? '' : p.data?.name ?? ''
      },
    },
    series: [
      {
        type: 'graph',
        layout: 'force',
        roam: true,
        label: { position: 'right', fontSize: 12, color: nodeLabelColor },
        edgeSymbol: ['none', 'arrow'],
        edgeSymbolSize: 6,
        edgeLabel: {
          show: true,
          fontSize: 11,
          color: edgeLabelColor,
          backgroundColor: isLight ? 'rgba(248, 250, 252, 0.82)' : 'rgba(15, 23, 42, 0.62)',
          padding: [2, 4],
          borderRadius: 4,
          formatter: '{c}',
        },
        data: nodes,
        links,
        force: {
          repulsion: 320,
          edgeLength: 130,
          gravity: 0.08,
        },
        emphasis: {
          scale: false,
          focus: 'adjacency',
          lineStyle: {
            color: edgeEmphasisColor,
            width: isLight ? 2.8 : 2.2,
          },
          itemStyle: {
            borderWidth: 2,
            borderColor: 'rgba(255,255,255,0.5)',
            shadowBlur: 16,
          },
          label: { color: '#ffffff' },
        },
        blur: {
          itemStyle: {
            opacity: 0.12,
            shadowBlur: 0,
            borderWidth: 0,
          },
          label: { opacity: 0.15 },
          lineStyle: { opacity: isLight ? 0.08 : 0.05 },
          edgeLabel: { opacity: 0 },
        },
      },
    ],
  }
})

function onChartClick(params: unknown) {
  const p = params as { dataType?: string; data?: { id?: string } }
  if (p.dataType === 'node' && p.data?.id) {
    const id = parseInt(p.data.id, 10)
    if (!Number.isNaN(id)) graphStore.setSelectedEntity(id)
  }
}

onMounted(() => {
  graphStore.fetchGraph()
  window.addEventListener('app-theme-changed', onThemeChanged)
})

onUnmounted(() => {
  window.removeEventListener('app-theme-changed', onThemeChanged)
  if (backgroundClickDisposer.value) {
    backgroundClickDisposer.value()
    backgroundClickDisposer.value = null
  }
})
</script>

<template>
  <div class="graph-panel">
    <div
      v-if="availableTypes.length"
      class="graph-filters"
    >
      <span class="filter-label">{{ t('graphPanel.filter') }}</span>
      <button
        type="button"
        class="filter-chip"
        :class="{ active: allTypesSelected }"
        @click="toggleAllTypes"
      >
        {{ t('graphPanel.all') }}
      </button>
      <button
        v-for="type in availableTypes"
        :key="type"
        type="button"
        class="filter-chip"
        :class="{ active: selectedTypes.includes(type) }"
        @click="toggleType(type)"
      >
        <span
          class="type-dot"
          :style="{ background: nodeColors[type] ?? '#22d3ee' }"
        />
        {{ typeDisplayName(type) }}
      </button>
    </div>
    <p v-if="graphStore.loading" class="loading">Loading…</p>
    <p v-else-if="graphStore.error" class="error">{{ graphStore.error }}</p>
    <v-chart
      v-else
      ref="chartRef"
      class="chart"
      :option="chartOption"
      autoresize
      @click="onChartClick"
      @finished="onChartFinished"
    />
  </div>
</template>

<style scoped>
.graph-panel {
  height: 100%;
  position: relative;
  background: transparent;
  display: flex;
  flex-direction: column;
}
.graph-filters {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg2) 82%, transparent);
  overflow-x: auto;
}
.graph-filters::-webkit-scrollbar {
  height: 4px;
}
.graph-filters::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 2px;
}
.filter-label {
  font-size: 11px;
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  white-space: nowrap;
}
.filter-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text-muted);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}
.filter-chip:hover {
  border-color: var(--border-hover);
  color: var(--text);
}
.filter-chip.active {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: color-mix(in srgb, var(--accent) 16%, var(--bg3));
  color: var(--text);
}
.type-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
}
.loading,
.error {
  padding: 1rem;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--text-muted);
}
.error { color: var(--red); }
.chart {
  flex: 1;
  width: 100%;
  min-height: 300px;
}
</style>
