<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'
import type { MemoryItem } from '../composables/useApi'

const api = useApi()
const toast = useToast()
const store = useAppStore()
const { t } = useI18n()

const searchInput = ref('')
const searchK = ref(10)
const searchMetric = ref('cs')
const results = ref<MemoryItem[]>([])
const searching = ref(false)
const searchTime = ref(0)
const selectedId = ref<number | null>(null)
const detailItem = ref<MemoryItem | null>(null)
const vectorExpanded = ref(false)
const deletingIds = ref(new Set<number>())

const insertVector = ref('')
const insertMetadata = ref([{ key: '', value: '' }])
const insertType = ref('semantic')
const inserting = ref(false)

const detailVectorExpanded = ref(false)

function parseVector(input: string): number[] | null {
  let s = input.trim()
  if (s.startsWith('[')) {
    try {
      const arr = JSON.parse(s)
      if (Array.isArray(arr) && arr.every(v => typeof v === 'number')) return arr
    } catch {}
  }
  const nums = s.split(',').map(v => parseFloat(v.trim())).filter(v => !isNaN(v))
  return nums.length > 0 ? nums : null
}

async function doSearch() {
  const vector = parseVector(searchInput.value)
  if (!vector) {
    toast.error(t('memories.enterVector'))
    return
  }
  searching.value = true
  const start = Date.now()
  try {
    results.value = await api.search(store.currentNs, {
      vector,
      k: searchK.value,
      metric: searchMetric.value,
    })
    searchTime.value = Date.now() - start
    selectedId.value = null
    detailItem.value = null
    if (results.value.length === 0) {
      toast.info(t('memories.noResults'))
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    searching.value = false
  }
}

async function selectItem(item: MemoryItem) {
  selectedId.value = item.id
  vectorExpanded.value = false
  detailVectorExpanded.value = false
  try {
    const [detail] = await Promise.all([
      api.getMemory(store.currentNs, item.id),
      api.accessMemory(store.currentNs, item.id).catch(() => {}),
    ])
    detailItem.value = {
      ...detail,
      access_count: (detail.access_count ?? item.access_count) + 1,
    }
  } catch (e: any) {
    detailItem.value = { ...item, access_count: item.access_count + 1 }
  }
}

function closeDetail() {
  detailItem.value = null
  selectedId.value = null
}

async function deleteItem(id: number, event: Event) {
  event.stopPropagation()
  if (!confirm(t('memories.confirmDelete', { id }))) return
  deletingIds.value.add(id)
  try {
    await api.deleteMemory(store.currentNs, id)
    results.value = results.value.filter(r => r.id !== id)
    if (selectedId.value === id) {
      selectedId.value = null
      detailItem.value = null
    }
    toast.success(`Deleted memory ${id}`)
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    deletingIds.value.delete(id)
  }
}

function toggleVector(_id: number, event: Event) {
  event.stopPropagation()
  vectorExpanded.value = !vectorExpanded.value
}

async function doInsert() {
  const vector = parseVector(insertVector.value)
  if (!vector) {
    toast.error(t('memories.enterVector'))
    return
  }
  if (store.info?.dimension && vector.length !== store.info.dimension) {
    toast.error(t('memories.dimensionMismatch', { expected: store.info.dimension, got: vector.length }))
    return
  }
  inserting.value = true
  const metadata: Record<string, string> = {}
  for (const m of insertMetadata.value) {
    if (m.key.trim()) metadata[m.key.trim()] = m.value
  }
  try {
    const result = await api.insertMemory(store.currentNs, { vector, metadata, memory_type: insertType.value })
    if (result.ok) {
      toast.success(t('memories.insertedSuccess', { id: result.id }))
      insertVector.value = ''
      insertMetadata.value = [{ key: '', value: '' }]
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    inserting.value = false
  }
}

function addMetaField() {
  insertMetadata.value.push({ key: '', value: '' })
}

function removeMetaField(i: number) {
  insertMetadata.value.splice(i, 1)
}
</script>

<template>
  <div class="flex gap-5 min-h-[calc(100vh-140px)]">
    <div class="flex flex-col gap-4 w-[340px] flex-shrink-0">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-3">
        <h3 class="m-0 text-sm font-semibold flex items-center gap-2">
          <font-awesome-icon icon="magnifying-glass" class="w-3.5 text-text-muted" />
          {{ t('memories.vectorSearch') }}
        </h3>
        <div class="relative">
          <font-awesome-icon icon="brackets-square" class="absolute left-3 top-1/2 -translate-y-1/2 w-3 text-text-muted" />
          <input
            v-model="searchInput"
            :placeholder="t('memories.searchPlaceholder')"
            class="w-full bg-bg-primary border border-border-color rounded-lg pl-8 pr-3 py-2.5 text-text-primary text-[13px] font-mono outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
            @keydown.enter="doSearch"
          />
        </div>
        <div class="flex gap-2">
          <div class="flex-1 flex flex-col gap-1">
            <label class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.topK') }}</label>
            <input
              v-model.number="searchK"
              type="number"
              min="1"
              max="100"
              class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
            />
          </div>
          <div class="flex-1 flex flex-col gap-1">
            <label class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.distanceMetric') }}</label>
            <select
              v-model="searchMetric"
              class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
            >
              <option value="cs">Cosine</option>
              <option value="eu">Euclidean</option>
              <option value="dot">Dot Product</option>
            </select>
          </div>
        </div>
        <button
          class="flex items-center justify-center gap-2 w-full bg-accent-primary text-white border-none rounded-lg px-4 py-2.5 text-[13px] font-semibold cursor-pointer transition-all duration-150 hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="searching"
          @click="doSearch"
        >
          <font-awesome-icon v-if="searching" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="magnifying-glass" class="w-3.5" />
          {{ searching ? t('memories.searchButton') + '...' : t('memories.searchButton') }}
        </button>
      </div>

      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-3">
        <h3 class="m-0 text-sm font-semibold flex items-center gap-2">
          <font-awesome-icon icon="plus" class="w-3.5 text-text-muted" />
          {{ t('memories.insertMemory') }}
        </h3>
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.vectorLabel') }}</label>
          <textarea
            v-model="insertVector"
            rows="3"
            :placeholder="t('memories.vectorPlaceholder')"
            class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] font-mono outline-none transition-all duration-150 focus:border-accent-primary resize-y placeholder:text-text-muted"
          ></textarea>
        </div>
        <div class="flex flex-col gap-1">
          <div class="flex items-center justify-between">
            <label class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.metadataLabel') }}</label>
            <button
              class="flex items-center gap-1 text-[10px] text-accent-primary bg-transparent border-none cursor-pointer hover:underline"
              @click="addMetaField"
            >
              <font-awesome-icon icon="plus" class="w-2.5" />
              {{ t('memories.addField') }}
            </button>
          </div>
          <div v-for="(m, i) in insertMetadata" :key="i" class="flex gap-1.5">
            <input
              v-model="m.key"
              :placeholder="t('memories.metadataKeyPlaceholder')"
              class="flex-1 bg-bg-primary border border-border-color rounded-lg px-2.5 py-1.5 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
            />
            <input
              v-model="m.value"
              :placeholder="t('memories.metadataValuePlaceholder')"
              class="flex-1 bg-bg-primary border border-border-color rounded-lg px-2.5 py-1.5 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
            />
            <button
              v-if="insertMetadata.length > 1"
              class="flex items-center justify-center w-7 bg-transparent border border-border-color rounded-lg text-text-muted cursor-pointer hover:border-red-500 hover:text-red-400 transition-all"
              @click="removeMetaField(i)"
            >
              <font-awesome-icon icon="xmark" class="w-3" />
            </button>
          </div>
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.memoryType') }}</label>
          <select
            v-model="insertType"
            class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
          >
            <option value="semantic">{{ t('memories.typeSemantic') }}</option>
            <option value="episodic">{{ t('memories.typeEpisodic') }}</option>
          </select>
        </div>
        <button
          class="flex items-center justify-center gap-2 w-full bg-accent-primary text-white border-none rounded-lg px-4 py-2.5 text-[13px] font-semibold cursor-pointer transition-all duration-150 hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="inserting"
          @click="doInsert"
        >
          <font-awesome-icon v-if="inserting" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="plus" class="w-3.5" />
          {{ inserting ? t('memories.inserting') : t('memories.insertButton') }}
        </button>
      </div>
    </div>

    <div class="flex-1 flex flex-col min-w-0">
      <div v-if="results.length > 0" class="flex items-center gap-2 mb-3">
        <span class="text-xs text-text-muted">{{ t('memories.found', { count: results.length, ms: searchTime }) }}</span>
      </div>
      <div class="flex gap-4 flex-1 min-h-0">
        <div class="flex-1 flex flex-col gap-2 overflow-auto pr-1 min-w-0">
          <div
            v-for="item in results"
            :key="item.id"
            class="bg-bg-secondary border rounded-xl p-4 cursor-pointer transition-all duration-150 relative group"
            :class="item.id === selectedId ? 'border-accent-primary shadow-md shadow-accent-primary/10' : 'border-border-color hover:border-border-hover'"
            @click="selectItem(item)"
          >
            <div class="flex items-start justify-between mb-2">
              <div class="flex items-center gap-2">
                <span class="font-mono text-[11px] text-text-muted">#{{ item.id }}</span>
                <span
                  class="px-1.5 py-0.5 rounded text-[10px] font-medium"
                  :class="item.memory_type === 'Semantic' ? 'bg-blue-900/50 text-blue-400' : 'bg-purple-900/50 text-purple-400'"
                >{{ item.memory_type }}</span>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="text-[11px] font-semibold text-accent-primary">{{ item.score.toFixed(4) }}</span>
                <button
                  class="flex items-center justify-center w-6 h-6 bg-transparent border-none text-text-muted rounded cursor-pointer opacity-0 group-hover:opacity-100 transition-all hover:bg-bg-tertiary hover:text-text-primary"
                  :title="vectorExpanded ? t('memories.collapseVector') : t('memories.expandVector')"
                  @click="(e: Event) => toggleVector(item.id, e)"
                >
                  <font-awesome-icon icon="code" class="w-3" />
                </button>
                <button
                  class="flex items-center justify-center w-6 h-6 bg-transparent border-none text-text-muted rounded cursor-pointer opacity-0 group-hover:opacity-100 transition-all hover:bg-red-950/50 hover:text-red-400"
                  :disabled="deletingIds.has(item.id)"
                  :title="t('memories.deleteMemory')"
                  @click="(e: Event) => deleteItem(item.id, e)"
                >
                  <font-awesome-icon v-if="deletingIds.has(item.id)" icon="circle-notch" class="w-3 animate-spin" />
                  <font-awesome-icon v-else icon="trash" class="w-3" />
                </button>
              </div>
            </div>
            <div class="flex flex-wrap gap-1.5 mb-2">
              <span
                v-for="(val, key) in item.metadata"
                :key="key"
                class="inline-flex items-center gap-1 px-2 py-0.5 bg-bg-tertiary rounded text-[11px]"
              >
                <span class="text-text-muted">{{ key }}:</span>
                <span class="text-text-primary font-medium">{{ val }}</span>
              </span>
              <span v-if="Object.keys(item.metadata).length === 0" class="text-text-muted text-[11px] italic">no metadata</span>
            </div>
            <div class="flex items-center gap-3 text-[10px] text-text-muted">
              <span>dist: {{ item.distance.toFixed(4) }}</span>
              <span>access: {{ item.access_count }}</span>
              <span>decay: {{ item.decay_score.toFixed(3) }}</span>
            </div>
            <div v-if="vectorExpanded && item.vector" class="mt-2 p-2 bg-bg-primary rounded text-[10px] font-mono text-text-secondary break-all leading-relaxed">
              [{{ item.vector.map(v => v.toFixed(4)).join(', ') }}]
            </div>
          </div>
          <div v-if="results.length === 0" class="flex flex-col items-center justify-center flex-1 text-text-muted">
            <font-awesome-icon icon="magnifying-glass" class="w-8 mb-3 opacity-30" />
            <span class="text-sm">{{ t('memories.noResults') }}</span>
          </div>
        </div>

        <div v-if="detailItem" class="w-[320px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl p-5 overflow-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="m-0 text-sm font-semibold flex items-center gap-2">
              <font-awesome-icon icon="brain" class="w-3.5 text-accent-primary" />
              Memory #{{ detailItem.id }}
            </h3>
            <button
              class="flex items-center justify-center w-6 h-6 bg-transparent border-none text-text-muted rounded cursor-pointer transition-all hover:bg-bg-tertiary hover:text-text-primary"
              :title="t('memories.closeDetails')"
              @click="closeDetail"
            >
              <font-awesome-icon icon="xmark" class="w-3.5" />
            </button>
          </div>
          <div class="flex flex-col gap-3">
            <div>
              <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">{{ t('memories.memoryType') }}</span>
              <span
                class="px-2 py-1 rounded text-xs font-medium"
                :class="detailItem.memory_type === 'Semantic' ? 'bg-blue-900/50 text-blue-400' : 'bg-purple-900/50 text-purple-400'"
              >{{ detailItem.memory_type }}</span>
            </div>
            <div>
              <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">Score</span>
              <span class="text-lg font-bold text-accent-primary">{{ detailItem.score.toFixed(4) }}</span>
            </div>
            <div>
              <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">Distance</span>
              <span class="text-sm font-mono text-text-primary">{{ detailItem.distance.toFixed(6) }}</span>
            </div>
            <div class="flex gap-4">
              <div>
                <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">{{ t('memories.accessCount') }}</span>
                <span class="text-sm font-semibold text-text-primary">{{ detailItem.access_count }}</span>
              </div>
              <div>
                <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">{{ t('memories.decayScore') }}</span>
                <span class="text-sm font-semibold text-text-primary">{{ detailItem.decay_score.toFixed(3) }}</span>
              </div>
            </div>
            <div>
              <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium block mb-1">Created</span>
              <span class="text-xs text-text-secondary">{{ new Date(detailItem.created_at).toLocaleString() }}</span>
            </div>
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium">{{ t('memories.metadataLabel') }}</span>
              </div>
              <div class="flex flex-col gap-1">
                <div
                  v-for="(val, key) in detailItem.metadata"
                  :key="key"
                  class="flex justify-between items-center py-1.5 px-2.5 bg-bg-tertiary rounded text-xs"
                >
                  <span class="text-text-muted font-mono">{{ key }}</span>
                  <span class="text-text-primary font-medium">{{ val }}</span>
                </div>
                <div v-if="Object.keys(detailItem.metadata).length === 0" class="text-text-muted text-[11px] italic py-2 text-center">No metadata</div>
              </div>
            </div>
            <div v-if="detailItem.vector">
              <div class="flex items-center justify-between mb-1">
                <span class="text-[10px] text-text-muted uppercase tracking-wider font-medium">Vector ({{ detailItem.vector.length }}d)</span>
                <button
                  class="text-[10px] text-accent-primary bg-transparent border-none cursor-pointer hover:underline"
                  @click="detailVectorExpanded = !detailVectorExpanded"
                >
                  {{ detailVectorExpanded ? t('memories.collapseVector') : t('memories.expandVector') }}
                </button>
              </div>
              <div class="p-2.5 bg-bg-primary rounded text-[10px] font-mono text-text-secondary break-all leading-relaxed max-h-[120px] overflow-auto">
                <template v-if="detailVectorExpanded">
                  [{{ detailItem.vector.map(v => v.toFixed(6)).join(', ') }}]
                </template>
                <template v-else>
                  [{{ detailItem.vector.slice(0, 5).map(v => v.toFixed(4)).join(', ') }}{{ detailItem.vector.length > 5 ? ', ...' : '' }}]
                </template>
              </div>
            </div>
          </div>
        </div>
        <div v-else-if="results.length > 0" class="w-[320px] flex-shrink-0 flex flex-col items-center justify-center bg-bg-secondary border border-dashed border-border-color rounded-xl">
          <font-awesome-icon icon="brain" class="w-8 text-text-muted opacity-30 mb-2" />
          <span class="text-xs text-text-muted">{{ t('memories.selectMemory') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
