<script setup lang="ts">
import { ref } from 'vue'
import { useApi } from '../composables/useApi'

const api = useApi()
const startId = ref<number | null>(null)
const traverseDepth = ref(2)
const edgeTypeFilter = ref('')
const nodes = ref<any[]>([])
const edges = ref<any[]>([])
const activation = ref<any[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

const nodePositions = ref<Record<number, { x: number; y: number }>>({})

const edgeTypes = [
  { value: '', label: '全部' },
  { value: 'similar_to', label: 'SimilarTo' },
  { value: 'derived_from', label: 'DerivedFrom' },
  { value: 'part_of', label: 'PartOf' },
  { value: 'temporally_after', label: 'TemporallyAfter' },
  { value: 'contradicts', label: 'Contradicts' },
  { value: 'references', label: 'References' },
]

async function doTraverse() {
  if (startId.value === null) return
  loading.value = true
  error.value = null
  try {
    const result = await api.graphTraverse({
      start: startId.value,
      depth: traverseDepth.value,
      edge_type: edgeTypeFilter.value || undefined,
    })
    nodes.value = result
    edges.value = []
    layoutNodes(result)
    for (const node of result) {
      try {
        const edgeList = await api.getEdges(node.id)
        edges.value.push(...edgeList)
      } catch {}
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function doActivate() {
  if (startId.value === null) return
  loading.value = true
  error.value = null
  try {
    activation.value = await api.spreadingActivation({
      start: startId.value,
      decay_factor: 0.5,
      threshold: 0.1,
      max_hops: 3,
    })
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function layoutNodes(nodeList: any[]) {
  const positions: Record<number, { x: number; y: number }> = {}
  const cx = 300
  const cy = 200
  const maxDepth = Math.max(...nodeList.map(n => n.depth), 1)

  for (const node of nodeList) {
    const depthRatio = node.depth / maxDepth
    const angle = (positions[0] ? Math.random() : 0) + node.id % 360
    const radius = 40 + depthRatio * 200
    positions[node.id] = {
      x: cx + radius * Math.cos(angle * Math.PI / 180),
      y: cy + radius * Math.sin(angle * Math.PI / 180),
    }
  }
  nodePositions.value = positions
}

function getEdgeColor(edgeType: string): string {
  const colors: Record<string, string> = {
    SimilarTo: '#58a6ff',
    DerivedFrom: '#bc8cff',
    PartOf: '#3fb950',
    TemporallyAfter: '#d29922',
    Contradicts: '#f85149',
    References: '#8b949e',
  }
  return colors[edgeType] || '#8b949e'
}

function getNodeColor(depth: number): string {
  const colors = ['#58a6ff', '#3fb950', '#d29922', '#bc8cff', '#f85149']
  return colors[depth % colors.length]
}

function getActivationOpacity(nodeId: number): number {
  const act = activation.value.find(a => a.id === nodeId)
  if (!act) return 0.6
  return 0.3 + act.activation * 0.7
}

</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl p-4">
      <div class="flex gap-2 flex-wrap items-center">
        <input
          v-model.number="startId"
          type="number"
          placeholder="起始节点 ID"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
        />
        <input
          v-model.number="traverseDepth"
          type="number"
          min="1"
          max="5"
          placeholder="深度"
          class="w-20 bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
        />
        <select
          v-model="edgeTypeFilter"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option v-for="et in edgeTypes" :key="et.value" :value="et.value">
            {{ et.label }}
          </option>
        </select>
        <button
          class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doTraverse"
        >
          遍历
        </button>
        <button
          class="px-4 py-2 border border-border-color rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-bg-tertiary text-text-primary hover:bg-bg-elevated disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doActivate"
        >
          扩散激活
        </button>
      </div>
    </div>

    <div v-if="error" class="px-4 py-2.5 bg-red-900/50 border border-red-600 rounded-lg text-red-400 text-sm">{{ error }}</div>

    <div class="flex gap-4 min-h-[500px]">
      <div class="flex-1 bg-bg-primary border border-border-color rounded-xl overflow-hidden">
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
            stroke-opacity="0.4"
          />
          <g v-for="node in nodes" :key="node.id">
            <circle
              :cx="nodePositions[node.id]?.x ?? 0"
              :cy="nodePositions[node.id]?.y ?? 0"
              :r="12 + (node.weight ?? 1) * 8"
              :fill="getNodeColor(node.depth)"
              :fill-opacity="getActivationOpacity(node.id)"
              stroke="#30363d"
              stroke-width="1.5"
            />
            <text
              :x="nodePositions[node.id]?.x ?? 0"
              :y="(nodePositions[node.id]?.y ?? 0) + 4"
              text-anchor="middle"
              fill="#e1e4e8"
              font-size="10"
              font-family="monospace"
            >
              {{ node.id }}
            </text>
          </g>
        </svg>
      </div>

      <div v-if="nodes.length > 0" class="w-[220px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl p-4">
        <h3 class="m-0 mb-2.5 text-sm text-text-primary">图例</h3>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#58a6ff]"></span>
          <span>SimilarTo</span>
        </div>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#bc8cff]"></span>
          <span>DerivedFrom</span>
        </div>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#3fb950]"></span>
          <span>PartOf</span>
        </div>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#d29922]"></span>
          <span>TemporallyAfter</span>
        </div>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#f85149]"></span>
          <span>Contradicts</span>
        </div>
        <div class="flex items-center gap-2 py-1 text-xs text-text-secondary">
          <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 bg-[#8b949e]"></span>
          <span>References</span>
        </div>

        <div v-if="activation.length > 0" class="mt-4 border-t border-border-color pt-3">
          <h3 class="m-0 mb-2 text-[13px] text-text-primary">激活结果</h3>
          <div v-for="act in activation.slice(0, 10)" :key="act.id" class="flex items-center gap-2 py-1">
            <span class="font-mono text-[11px] text-accent-primary min-w-[50px]">#{{ act.id }}</span>
            <div class="flex-1 h-1.5 bg-bg-tertiary rounded-sm overflow-hidden">
              <div
                class="h-full rounded-sm bg-gradient-to-r from-accent-primary to-accent-secondary transition-all duration-300"
                :style="{ width: (act.activation * 100) + '%' }"
              ></div>
            </div>
            <span class="font-mono text-[11px] text-text-secondary min-w-[40px] text-right">{{ act.activation.toFixed(3) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
