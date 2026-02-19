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
    return { title: { text: 'No graph data', left: 'center' } }
  }
  const nodeColors: Record<string, string> = {
    Person: '#5470c6',
    Location: '#91cc75',
    Event: '#fac858',
    Time: '#ee6666',
  }
  const nodes = g.nodes.map((n: { id: string; name: string; type: string }) => ({
    id: n.id,
    name: n.name,
    symbolSize: 35,
    itemStyle: { color: nodeColors[n.type] ?? '#999' },
    label: { show: true, fontSize: 12 },
  }))
  const links = g.links.map((l: { source: string; target: string; relation: string; strength: number }) => ({
    source: l.source,
    target: l.target,
    value: l.relation,
    lineStyle: { width: Math.max(1, Math.min(l.strength, 4)) },
  }))
  return {
    title: { text: 'Knowledge Graph', left: 'center', top: 8, textStyle: { fontSize: 16 } },
    tooltip: {
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
        label: { position: 'right', fontSize: 12 },
        edgeSymbol: ['none', 'arrow'],
        edgeSymbolSize: 8,
        edgeLabel: {
          show: true,
          fontSize: 10,
          formatter: '{c}',
        },
        data: nodes,
        links,
        force: {
          repulsion: 300,
          edgeLength: 120,
          gravity: 0.1,
        },
        emphasis: { focus: 'adjacency' },
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
}
.loading,
.error {
  padding: 1rem;
  margin: 0;
  font-size: 0.875rem;
}
.error {
  color: var(--color-error, #c00);
}
.chart {
  width: 100%;
  height: 100%;
  min-height: 300px;
}
</style>
