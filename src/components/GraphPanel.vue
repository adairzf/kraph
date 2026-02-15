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
    return { title: { text: '暂无图谱数据', left: 'center' } }
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
    symbolSize: 28,
    itemStyle: { color: nodeColors[n.type] ?? '#999' },
    label: { show: true },
  }))
  const links = g.links.map((l: { source: string; target: string; relation: string; strength: number }) => ({
    source: l.source,
    target: l.target,
    value: l.relation,
    lineStyle: { width: Math.max(1, Math.min(l.strength, 4)) },
  }))
  return {
    title: { text: '知识图谱', left: 'center', top: 8 },
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
        label: { position: 'right' },
        edgeSymbol: ['none', 'none'],
        data: nodes,
        links,
        force: {
          repulsion: 200,
          edgeLength: 80,
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
    <p v-if="graphStore.loading" class="loading">加载中…</p>
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
