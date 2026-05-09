<script setup lang="ts">
import { ref, onMounted } from 'vue'
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

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString('zh-CN')
}
</script>

<template>
  <div class="graph-view">
    <div class="toolbar">
      <div class="toolbar-row">
        <input
          v-model.number="startId"
          type="number"
          placeholder="起始节点 ID"
          class="input-field"
        />
        <input
          v-model.number="traverseDepth"
          type="number"
          min="1"
          max="5"
          class="input-field narrow"
          placeholder="深度"
        />
        <select v-model="edgeTypeFilter" class="input-field">
          <option v-for="et in edgeTypes" :key="et.value" :value="et.value">
            {{ et.label }}
          </option>
        </select>
        <button class="btn btn-primary" :disabled="loading" @click="doTraverse">
          遍历
        </button>
        <button class="btn btn-secondary" :disabled="loading" @click="doActivate">
          扩散激活
        </button>
      </div>
    </div>

    <div v-if="error" class="alert error">{{ error }}</div>

    <div class="graph-container">
      <div class="canvas-wrapper">
        <svg width="100%" height="500" class="graph-svg">
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

      <div v-if="nodes.length > 0" class="legend-panel">
        <h3>图例</h3>
        <div class="legend-item">
          <span class="legend-circle" style="background:#58a6ff"></span>
          <span>SimilarTo</span>
        </div>
        <div class="legend-item">
          <span class="legend-circle" style="background:#bc8cff"></span>
          <span>DerivedFrom</span>
        </div>
        <div class="legend-item">
          <span class="legend-circle" style="background:#3fb950"></span>
          <span>PartOf</span>
        </div>
        <div class="legend-item">
          <span class="legend-circle" style="background:#d29922"></span>
          <span>TemporallyAfter</span>
        </div>
        <div class="legend-item">
          <span class="legend-circle" style="background:#f85149"></span>
          <span>Contradicts</span>
        </div>
        <div class="legend-item">
          <span class="legend-circle" style="background:#8b949e"></span>
          <span>References</span>
        </div>

        <div v-if="activation.length > 0" class="activation-section">
          <h3>激活结果</h3>
          <div v-for="act in activation.slice(0, 10)" :key="act.id" class="activation-row">
            <span class="act-id">#{{ act.id }}</span>
            <div class="act-bar">
              <div class="act-fill" :style="{ width: (act.activation * 100) + '%' }"></div>
            </div>
            <span class="act-val">{{ act.activation.toFixed(3) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.graph-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.toolbar {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 16px;
}

.toolbar-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.input-field {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 8px 12px;
  color: #e1e4e8;
  font-size: 13px;
  outline: none;
}

.input-field:focus {
  border-color: #58a6ff;
}

.input-field.narrow {
  width: 80px;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #238636;
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: #2ea043;
}

.btn-secondary {
  background: #21262d;
  color: #e1e4e8;
  border: 1px solid #30363d;
}

.btn-secondary:hover:not(:disabled) {
  background: #30363d;
}

.alert.error {
  padding: 10px 16px;
  background: #3d1414;
  border: 1px solid #da3633;
  border-radius: 8px;
  color: #f85149;
  font-size: 13px;
}

.graph-container {
  display: flex;
  gap: 16px;
  min-height: 500px;
}

.canvas-wrapper {
  flex: 1;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow: hidden;
}

.graph-svg {
  display: block;
}

.legend-panel {
  width: 220px;
  flex-shrink: 0;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 16px;
}

.legend-panel h3 {
  margin: 0 0 10px 0;
  font-size: 14px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
  color: #8b949e;
}

.legend-circle {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.activation-section {
  margin-top: 16px;
  border-top: 1px solid #30363d;
  padding-top: 12px;
}

.activation-section h3 {
  margin: 0 0 8px 0;
  font-size: 13px;
}

.activation-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}

.act-id {
  font-family: monospace;
  font-size: 11px;
  color: #58a6ff;
  min-width: 50px;
}

.act-bar {
  flex: 1;
  height: 6px;
  background: #21262d;
  border-radius: 3px;
  overflow: hidden;
}

.act-fill {
  height: 100%;
  background: linear-gradient(90deg, #58a6ff, #bc8cff);
  border-radius: 3px;
  transition: width 0.3s;
}

.act-val {
  font-family: monospace;
  font-size: 11px;
  color: #8b949e;
  min-width: 40px;
  text-align: right;
}
</style>
