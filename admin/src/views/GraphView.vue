<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const store = useAppStore()
const startId = ref<number | null>(null)
const traverseDepth = ref(2)
const edgeTypeFilter = ref('')
const nodes = ref<any[]>([])
const edges = ref<any[]>([])
const activation = ref<any[]>([])
const traversing = ref(false)
const activating = ref(false)

const nodePositions = ref<Record<number, { x: number; y: number }>>({})

const edgeTypes = computed(() => [
  { value: '', label: t('graph.allEdgeTypes') },
  { value: 'similar_to', label: 'SimilarTo' },
  { value: 'derived_from', label: 'DerivedFrom' },
  { value: 'part_of', label: 'PartOf' },
  { value: 'temporally_after', label: 'TemporallyAfter' },
  { value: 'contradicts', label: 'Contradicts' },
  { value: 'references', label: 'References' },
])

const loading = computed(() => traversing.value || activating.value)

async function doTraverse() {
  if (startId.value === null) {
    toast.error(t('graph.enterStartId'))
    return
  }
  traversing.value = true
  try {
    const result = await api.graphTraverse(store.currentNs, {
      start: startId.value,
      depth: traverseDepth.value,
      edge_type: edgeTypeFilter.value || undefined,
    })
    nodes.value = result
    edges.value = []
    layoutNodes(result)
    const edgeLists = await Promise.all(
      result.map(node => api.getEdges(store.currentNs, node.id).catch(() => []))
    )
    edges.value = edgeLists.flat()
    toast.info(t('graph.traversed', { nodes: result.length, edges: edges.value.length }))
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    traversing.value = false
  }
}

async function doActivate() {
  if (startId.value === null) {
    toast.error(t('graph.enterStartId'))
    return
  }
  activating.value = true
  try {
    activation.value = await api.spreadingActivation(store.currentNs, {
      start: startId.value,
      decay_factor: 0.5,
      threshold: 0.1,
      max_hops: 3,
    })
    toast.info(t('graph.activationComplete', { count: activation.value.length }))
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    activating.value = false
  }
}

function layoutNodes(nodeList: any[]) {
  const positions: Record<number, { x: number; y: number }> = {}
  const cx = 300
  const cy = 200
  const maxDepth = Math.max(...nodeList.map(n => n.depth), 1)
  const depthGroups: Record<number, number[]> = {}
  for (const node of nodeList) {
    if (!depthGroups[node.depth]) depthGroups[node.depth] = []
    depthGroups[node.depth].push(node.id)
  }
  for (const [depth, ids] of Object.entries(depthGroups)) {
    const d = Number(depth)
    const radius = 40 + (d / maxDepth) * 200
    ids.forEach((id, i) => {
      const angle = (2 * Math.PI * i) / ids.length + d * 0.5
      positions[id] = {
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
      }
    })
  }
  nodePositions.value = positions
}

function getEdgeColor(edgeType: string): string {
  const colors: Record<string, string> = {
    SimilarTo: '#f6821f',
    DerivedFrom: '#fbad41',
    PartOf: '#3fb950',
    TemporallyAfter: '#eab308',
    Contradicts: '#ef4444',
    References: '#6b6c72',
  }
  return colors[edgeType] || '#6b6c72'
}

function getNodeColor(depth: number): string {
  const colors = ['#f6821f', '#fbad41', '#3fb950', '#3b82f6', '#a855f7']
  return colors[depth % colors.length]
}

function getActivationOpacity(nodeId: number): number {
  const act = activation.value.find(a => a.id === nodeId)
  if (!act) return 0.6
  return 0.3 + act.activation * 0.7
}

const legendItems = [
  { label: 'SimilarTo', color: '#f6821f' },
  { label: 'DerivedFrom', color: '#fbad41' },
  { label: 'PartOf', color: '#3fb950' },
  { label: 'TemporallyAfter', color: '#eab308' },
  { label: 'Contradicts', color: '#ef4444' },
  { label: 'References', color: '#6b6c72' },
]
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl p-4">
      <div class="flex gap-2 flex-wrap items-center">
        <div class="relative">
          <font-awesome-icon icon="hashtag" class="absolute left-3 top-1/2 -translate-y-1/2 w-3 text-text-muted" />
          <input
            v-model.number="startId"
            type="number"
            :placeholder="t('graph.startNodeId')"
            class="w-36 bg-bg-primary border border-border-color rounded-lg pl-8 pr-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
          />
        </div>
        <input
          v-model.number="traverseDepth"
          type="number"
          min="1"
          max="5"
          :placeholder="t('graph.traversalDepth')"
          class="w-20 bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
        />
        <select
          v-model="edgeTypeFilter"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option v-for="et in edgeTypes" :key="et.value" :value="et.value">
            {{ et.label }}
          </option>
        </select>
        <button
          class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doTraverse"
        >
          <font-awesome-icon v-if="traversing" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="diagram-project" class="w-3.5" />
          {{ traversing ? t('graph.traversing') : t('graph.traverse') }}
        </button>
        <button
          class="flex items-center gap-2 px-3 py-2 border border-border-color rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-bg-tertiary text-text-secondary hover:border-border-hover hover:text-text-primary disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doActivate"
        >
          <font-awesome-icon v-if="activating" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="wave-square" class="w-3.5" />
          {{ activating ? t('graph.activating') : t('graph.activateButton') }}
        </button>
      </div>
    </div>

    <div class="flex gap-4 min-h-[500px]">
      <div class="flex-1 bg-bg-secondary border border-border-color rounded-xl overflow-hidden">
        <svg width="100%" height="500" class="block">
          <line
            v-for="(edge, i) in edges"
            :key="'e' + i"
            :x1="nodePositions[edge.from]?.x ?? 0"
            :y1="nodePositions[edge.from]?.y ?? 0"
            :x2="nodePositions[edge.to]?.x ?? 0"
            :y2="nodePositions[edge.to]?.y ?? 0"
            :stroke="getEdgeColor(edge.edge_type)"
            stroke-width="1.5"
            stroke-opacity="0.3"
          />
          <g v-for="node in nodes" :key="node.id">
            <circle
              :cx="nodePositions[node.id]?.x ?? 0"
              :cy="nodePositions[node.id]?.y ?? 0"
              :r="12 + (node.weight ?? 1) * 8"
              :fill="getNodeColor(node.depth)"
              :fill-opacity="getActivationOpacity(node.id)"
              stroke="#2a2b2e"
              stroke-width="1.5"
            />
            <text
              :x="nodePositions[node.id]?.x ?? 0"
              :y="(nodePositions[node.id]?.y ?? 0) + 4"
              text-anchor="middle"
              fill="#f0f1f2"
              font-size="10"
              font-family="JetBrains Mono, monospace"
            >
              {{ node.id }}
            </text>
          </g>
        </svg>
      </div>

      <div v-if="nodes.length > 0" class="w-[220px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl p-4">
        <h3 class="m-0 mb-3 text-xs font-semibold uppercase tracking-wider text-text-muted">{{ t('graph.legend') }}</h3>
        <div v-for="item in legendItems" :key="item.label" class="flex items-center gap-2.5 py-1.5">
          <span class="w-2 h-2 rounded-full flex-shrink-0" :style="{ background: item.color }"></span>
          <span class="text-xs text-text-secondary">{{ item.label }}</span>
        </div>

        <div v-if="activation.length > 0" class="mt-4 border-t border-border-color pt-3">
          <h3 class="m-0 mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">{{ t('graph.activation') }}</h3>
          <div v-for="act in activation.slice(0, 10)" :key="act.id" class="flex items-center gap-2 py-1">
            <span class="font-mono text-[10px] text-accent-primary min-w-[40px]">#{{ act.id }}</span>
            <div class="flex-1 h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
              <div
                class="h-full rounded-full bg-gradient-to-r from-accent-primary to-accent-secondary transition-all duration-300"
                :style="{ width: (act.activation * 100) + '%' }"
              ></div>
            </div>
            <span class="font-mono text-[10px] text-text-muted min-w-[36px] text-right">{{ act.activation.toFixed(3) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
