<script setup lang="ts">
import { computed, onMounted } from 'vue'
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

use([
  CanvasRenderer,
  GraphChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
])

const graphStore = useGraphStore()

const chartOption = computed(() => {
  const g = graphStore.graphData
  if (!g || (!g.nodes.length && !g.links.length)) {
    return {
      backgroundColor: 'transparent',
      title: {
        text: 'No graph data',
        left: 'center',
        top: 'middle',
        textStyle: { color: '#4a4a5e', fontSize: 13, fontWeight: 'normal' },
      },
    }
  }
  const nodeColors: Record<string, string> = {
    Person: '#7c5cfc',
    Location: '#34d399',
    Event: '#f472b6',
    Time: '#fb923c',
    Organization: '#3b82f6',
  }
  const nodes = g.nodes.map((n: { id: string; name: string; type: string }) => ({
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
      fontSize: 11,
      color: '#e2e2ee',
      fontFamily: 'Inter, -apple-system, sans-serif',
    },
  }))
  const links = g.links.map((l: { source: string; target: string; relation: string; strength: number }) => ({
    source: l.source,
    target: l.target,
    value: l.relation,
    lineStyle: {
      width: Math.max(1, Math.min(l.strength, 3)),
      color: 'rgba(255,255,255,0.12)',
    },
  }))
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
        label: { position: 'right', fontSize: 11, color: '#e2e2ee' },
        edgeSymbol: ['none', 'arrow'],
        edgeSymbolSize: 6,
        edgeLabel: {
          show: true,
          fontSize: 10,
          color: '#7a7a8e',
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
          focus: 'adjacency',
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
          lineStyle: { opacity: 0.05 },
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
})
</script>

<template>
  <div class="graph-panel">
    <p v-if="graphStore.loading" class="loading">Loading…</p>
    <p v-else-if="graphStore.error" class="error">{{ graphStore.error }}</p>
    <v-chart v-else class="chart" :option="chartOption" autoresize @click="onChartClick" />
  </div>
</template>

<style scoped>
.graph-panel {
  height: 100%;
  position: relative;
  background: transparent;
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
  width: 100%;
  height: 100%;
  min-height: 300px;
}
</style>
