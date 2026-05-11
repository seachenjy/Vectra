<script setup lang="ts">
import { ref } from 'vue'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'
import type { MemoryItem } from '../composables/useApi'

const api = useApi()
const toast = useToast()
const store = useAppStore()
const results = ref<MemoryItem[]>([])
const selectedId = ref<number | null>(null)
const detailItem = ref<MemoryItem | null>(null)
const showAllVector = ref(false)

const searchVector = ref('')
const searchK = ref(10)
const searchMetric = ref('cs')

const showInsertForm = ref(false)
const insertVector = ref('')
const insertMeta = ref('')
const insertType = ref('semantic')

const searching = ref(false)
const inserting = ref(false)
const deletingIds = ref(new Set<number>())

async function doSearch() {
  searching.value = true
  try {
    const vector = searchVector.value.split(',').map(Number).filter(n => !isNaN(n))
    if (vector.length === 0) {
      toast.error('Enter a valid vector separated by commas')
      return
    }
    if (store.info?.dimension && vector.length !== store.info.dimension) {
      toast.error(`Dimension mismatch: got ${vector.length}, expected ${store.info.dimension}`)
      return
    }
    results.value = await api.search(store.currentNs, {
      vector,
      k: searchK.value,
      metric: searchMetric.value,
    })
    toast.info(`Found ${results.value.length} memories`)
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    searching.value = false
  }
}

async function viewDetail(item: MemoryItem) {
  selectedId.value = item.id
  showAllVector.value = false
  try {
    const [detail] = await Promise.all([
      api.getMemory(store.currentNs, item.id),
      api.accessMemory(store.currentNs, item.id).catch(() => {}),
    ])
    detailItem.value = { ...detail, access_count: detail.access_count + 1 }
    const idx = results.value.findIndex(r => r.id === item.id)
    if (idx !== -1) results.value[idx] = { ...results.value[idx], access_count: results.value[idx].access_count + 1 }
  } catch (e: any) {
    toast.error(e.message)
  }
}

function closeDetail() {
  detailItem.value = null
  selectedId.value = null
}

async function deleteItem(id: number) {
  if (!confirm('Delete this memory?')) return
  deletingIds.value.add(id)
  try {
    await api.deleteMemory(store.currentNs, id)
    results.value = results.value.filter(r => r.id !== id)
    if (detailItem.value?.id === id) closeDetail()
    toast.success(`Memory #${id} deleted`)
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    deletingIds.value.delete(id)
  }
}

async function doInsert() {
  inserting.value = true
  try {
    const vector = insertVector.value.split(',').map(Number).filter(n => !isNaN(n))
    if (vector.length === 0) {
      toast.error('Enter a valid vector')
      return
    }
    if (store.info?.dimension && vector.length !== store.info.dimension) {
      toast.error(`Dimension mismatch: got ${vector.length}, expected ${store.info.dimension}`)
      return
    }
    const metadata: Record<string, string> = {}
    for (const line of insertMeta.value.split('\n')) {
      const trimmed = line.trim()
      if (!trimmed) continue
      const eqIdx = trimmed.indexOf('=')
      if (eqIdx > 0) {
        metadata[trimmed.slice(0, eqIdx).trim()] = trimmed.slice(eqIdx + 1).trim()
      }
    }
    const result = await api.insertMemory(store.currentNs, { vector, metadata, memory_type: insertType.value })
    toast.success(`Memory inserted, ID: ${result.id}`)
    showInsertForm.value = false
    insertVector.value = ''
    insertMeta.value = ''
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    inserting.value = false
  }
}

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString('zh-CN')
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl p-4">
      <div class="flex gap-2 flex-wrap items-center">
        <div class="relative flex-1 min-w-[200px]">
          <font-awesome-icon icon="magnifying-glass" class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 text-text-muted" />
          <input
            v-model="searchVector"
            placeholder="Vector (e.g. 0.1,0.2,0.3,...)"
            class="w-full bg-bg-primary border border-border-color rounded-lg pl-9 pr-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
            @keydown.enter="doSearch"
          />
        </div>
        <select
          v-model="searchMetric"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option value="cs">Cosine</option>
          <option value="eu">Euclidean</option>
          <option value="dot">Dot Product</option>
        </select>
        <input
          v-model.number="searchK"
          type="number"
          min="1"
          max="100"
          class="w-[60px] bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
          placeholder="K"
        />
        <button
          class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="searching"
          @click="doSearch"
        >
          <font-awesome-icon v-if="searching" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="magnifying-glass" class="w-3.5" />
          {{ searching ? 'Searching...' : 'Search' }}
        </button>
        <button
          class="flex items-center gap-2 px-3 py-2 border border-border-color rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-bg-tertiary text-text-secondary hover:border-border-hover hover:text-text-primary disabled:opacity-50 disabled:cursor-not-allowed"
          @click="showInsertForm = !showInsertForm"
        >
          <font-awesome-icon :icon="showInsertForm ? 'xmark' : 'plus'" class="w-3.5" />
          {{ showInsertForm ? 'Cancel' : 'Insert' }}
        </button>
      </div>
    </div>

    <div v-if="showInsertForm" class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-3">
      <div class="flex items-center gap-2 mb-1">
        <font-awesome-icon icon="plus" class="w-3.5 text-accent-primary" />
        <h3 class="m-0 text-sm font-semibold">Insert Memory</h3>
      </div>
      <input
        v-model="insertVector"
        placeholder="Vector (comma-separated)"
        class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
      />
      <textarea
        v-model="insertMeta"
        placeholder="Metadata (one key=value per line)"
        class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary resize-y font-mono placeholder:text-text-muted"
        rows="3"
      ></textarea>
      <div class="flex gap-2">
        <select
          v-model="insertType"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option value="semantic">Semantic</option>
          <option value="episodic">Episodic</option>
        </select>
        <button
          class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="inserting"
          @click="doInsert"
        >
          <font-awesome-icon v-if="inserting" icon="circle-notch" class="w-3.5 animate-spin" />
          {{ inserting ? 'Submitting...' : 'Submit' }}
        </button>
      </div>
    </div>

    <div class="flex gap-4 min-h-[400px]">
      <div class="flex-1 flex flex-col gap-2 overflow-auto">
        <div v-if="results.length === 0 && !searching" class="text-text-muted text-center py-20 text-sm">
          Enter a vector to search memories
        </div>
        <div
          v-for="item in results"
          :key="item.id"
          class="group bg-bg-secondary border rounded-lg p-3.5 cursor-pointer transition-all duration-150 hover:border-border-hover"
          :class="selectedId === item.id ? 'border-accent-primary bg-bg-tertiary' : 'border-border-color'"
          @click="viewDetail(item)"
        >
          <div class="flex items-center gap-2 mb-2">
            <span class="font-mono text-xs font-medium text-accent-primary">#{{ item.id }}</span>
            <span
              class="px-1.5 py-0.5 rounded text-[10px] font-medium uppercase tracking-wide"
              :class="item.memory_type === 'Episodic' ? 'bg-purple-950/60 text-purple-400 border border-purple-800/30' : 'bg-orange-950/60 text-orange-400 border border-orange-800/30'"
            >
              {{ item.memory_type }}
            </span>
            <button
              class="ml-auto flex items-center justify-center w-6 h-6 border-none bg-transparent rounded cursor-pointer text-text-muted transition-all duration-150 opacity-0 group-hover:opacity-100 hover:bg-red-950/60 hover:text-red-400 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="deletingIds.has(item.id)"
              @click.stop="deleteItem(item.id)"
              title="Delete"
            >
              <font-awesome-icon v-if="deletingIds.has(item.id)" icon="circle-notch" class="w-3 animate-spin" />
              <font-awesome-icon v-else icon="trash" class="w-3" />
            </button>
          </div>
          <div class="flex gap-4 text-[11px] text-text-muted font-mono">
            <span>score: {{ item.score.toFixed(4) }}</span>
            <span>dist: {{ item.distance.toFixed(4) }}</span>
            <span>access: {{ item.access_count }}</span>
          </div>
          <div v-if="Object.keys(item.metadata).length > 0" class="flex flex-wrap gap-1 mt-2">
            <span v-for="(v, k) in item.metadata" :key="k" class="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] text-text-muted font-mono">
              {{ k }}={{ v }}
            </span>
          </div>
        </div>
      </div>

      <div v-if="detailItem" class="w-[340px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl overflow-hidden">
        <div class="flex justify-between items-center px-5 py-3.5 border-b border-border-color">
          <div class="flex items-center gap-2">
            <font-awesome-icon icon="brain" class="w-3.5 text-accent-primary" />
            <h3 class="m-0 text-sm font-semibold">Memory #{{ detailItem.id }}</h3>
          </div>
          <button
            class="flex items-center justify-center w-6 h-6 border-none bg-transparent rounded cursor-pointer text-text-muted transition-colors hover:text-text-primary"
            @click="closeDetail"
          >
            <font-awesome-icon icon="xmark" class="w-3.5" />
          </button>
        </div>
        <div class="p-5 flex flex-col gap-4 overflow-auto max-h-[calc(100vh-200px)]">
          <div>
            <div class="text-[10px] font-medium uppercase tracking-wider text-text-muted mb-2">Vector</div>
            <div class="font-mono text-[11px] text-text-secondary bg-bg-primary p-2.5 rounded-lg break-all border border-border-color">
              <span v-if="!showAllVector">
                [{{ detailItem.vector.slice(0, 8).map(v => v.toFixed(4)).join(', ') }}{{ detailItem.vector.length > 8 ? ', ...' : '' }}]
                <button
                  v-if="detailItem.vector.length > 8"
                  class="ml-1 text-accent-primary bg-transparent border-none cursor-pointer text-[11px] hover:underline"
                  @click="showAllVector = true"
                >
                  expand all {{ detailItem.vector.length }}
                </button>
              </span>
              <span v-else>
                [{{ detailItem.vector.map(v => v.toFixed(4)).join(', ') }}]
                <button
                  class="ml-1 text-accent-primary bg-transparent border-none cursor-pointer text-[11px] hover:underline"
                  @click="showAllVector = false"
                >
                  collapse
                </button>
              </span>
            </div>
          </div>
          <div>
            <div class="text-[10px] font-medium uppercase tracking-wider text-text-muted mb-2">Metadata</div>
            <div v-for="(v, k) in detailItem.metadata" :key="k" class="flex justify-between py-1.5 border-b border-border-color last:border-0">
              <span class="text-xs text-text-muted">{{ k }}</span>
              <span class="text-xs text-text-primary font-mono break-all max-w-[180px] text-right">{{ v }}</span>
            </div>
          </div>
          <div>
            <div class="text-[10px] font-medium uppercase tracking-wider text-text-muted mb-2">Details</div>
            <div class="flex flex-col gap-0">
              <div class="flex justify-between items-center py-1.5 border-b border-border-color">
                <span class="text-xs text-text-muted">Created</span>
                <span class="text-xs text-text-primary font-mono">{{ formatTimestamp(detailItem.created_at) }}</span>
              </div>
              <div class="flex justify-between items-center py-1.5 border-b border-border-color">
                <span class="text-xs text-text-muted">Access Count</span>
                <span class="text-xs text-text-primary font-mono">{{ detailItem.access_count }}</span>
              </div>
              <div class="flex justify-between items-center py-1.5">
                <span class="text-xs text-text-muted">Decay Score</span>
                <span class="text-xs text-text-primary font-mono">{{ detailItem.decay_score.toFixed(4) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
