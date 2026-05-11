<script setup lang="ts">
import { ref } from 'vue'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'

const api = useApi()
const toast = useToast()
const store = useAppStore()

const queryText = ref('BOOST recent')
const queryK = ref(10)
const queryMetric = ref('cs')
const results = ref<any[]>([])
const loading = ref(false)
const queryHistory = ref<string[]>(loadHistory())

const examples = [
  { label: 'Recent memories', query: 'BOOST recent', icon: 'clock-rotate-left' },
  { label: 'Semantic search', query: 'VECTOR_SIMILAR("text")', icon: 'magnifying-glass' },
  { label: 'Property filter', query: 'category = technology', icon: 'filter' },
  { label: 'Combined query', query: 'VECTOR_SIMILAR("memory") AND category = important BOOST recent 2.0', icon: 'code' },
  { label: 'Graph expansion', query: 'EXPAND depth=2', icon: 'diagram-project' },
  { label: 'OR query', query: 'VECTOR_SIMILAR("test") OR category = demo', icon: 'code-branch' },
]

function loadHistory(): string[] {
  try {
    const stored = localStorage.getItem('skymemory_query_history')
    return stored ? JSON.parse(stored) : []
  } catch {
    return []
  }
}

function saveHistory(history: string[]) {
  localStorage.setItem('skymemory_query_history', JSON.stringify(history))
}

async function doQuery() {
  if (!queryText.value.trim()) {
    toast.error('Enter a query')
    return
  }
  loading.value = true
  try {
    results.value = await api.queryExec(store.currentNs, {
      query: queryText.value,
      k: queryK.value,
      metric: queryMetric.value,
    })
    if (!queryHistory.value.includes(queryText.value)) {
      queryHistory.value.unshift(queryText.value)
      if (queryHistory.value.length > 20) queryHistory.value.pop()
      saveHistory(queryHistory.value)
    }
    toast.info(`Query complete: ${results.value.length} results`)
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    loading.value = false
  }
}

function loadExample(query: string) {
  queryText.value = query
}

function loadFromHistory(query: string) {
  queryText.value = query
}

function clearHistory() {
  queryHistory.value = []
  localStorage.removeItem('skymemory_query_history')
  toast.info('History cleared')
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl overflow-hidden">
      <div class="flex justify-between items-center px-5 py-3 border-b border-border-color">
        <div class="flex items-center gap-2">
          <font-awesome-icon icon="code" class="w-3.5 text-accent-primary" />
          <h3 class="m-0 text-sm font-semibold">Semantic Query</h3>
        </div>
        <div class="flex gap-1.5 flex-wrap">
          <span v-for="tag in ['VECTOR_SIMILAR', 'key = value', 'AND / OR', 'BOOST', 'EXPAND']" :key="tag"
            class="px-2 py-0.5 bg-bg-tertiary border border-border-color rounded text-[10px] font-mono text-text-muted">
            {{ tag }}
          </span>
        </div>
      </div>
      <div class="p-4">
        <textarea
          v-model="queryText"
          placeholder="Enter query expression..."
          class="w-full bg-bg-primary border border-border-color rounded-lg px-4 py-3 text-text-primary text-[13px] font-mono resize-y outline-none leading-6 transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
          rows="4"
          @keydown.ctrl.enter="doQuery"
        ></textarea>
      </div>
      <div class="flex justify-between items-center px-5 py-3 border-t border-border-color bg-bg-tertiary/50">
        <div class="flex gap-2 items-center">
          <select
            v-model="queryMetric"
            class="bg-bg-primary border border-border-color rounded-lg px-3 py-1.5 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
          >
            <option value="cs">Cosine</option>
            <option value="eu">Euclidean</option>
            <option value="dot">Dot Product</option>
          </select>
          <input
            v-model.number="queryK"
            type="number"
            min="1"
            max="100"
            class="w-[60px] bg-bg-primary border border-border-color rounded-lg px-3 py-1.5 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
            placeholder="K"
          />
        </div>
        <button
          class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doQuery"
        >
          <font-awesome-icon v-if="loading" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="magnifying-glass" class="w-3.5" />
          {{ loading ? 'Executing...' : 'Execute' }}
          <span class="text-[11px] opacity-60 ml-1">Ctrl+Enter</span>
        </button>
      </div>
    </div>

    <div class="flex gap-4 min-h-[400px]">
      <div class="flex-1 flex flex-col gap-2">
        <div class="flex items-center gap-2 mb-1">
          <span class="text-xs font-semibold uppercase tracking-wider text-text-muted">Results</span>
          <span class="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] font-mono text-text-muted">{{ results.length }}</span>
        </div>
        <div v-if="results.length === 0 && !loading" class="text-text-muted text-center py-20 text-sm">
          Execute a query to see results
        </div>
        <div v-for="(item, i) in results" :key="i" class="bg-bg-secondary border border-border-color rounded-lg px-4 py-3 transition-all duration-150 hover:border-border-hover">
          <div class="flex items-center gap-2.5 mb-2">
            <span class="font-mono text-xs font-medium text-accent-primary">#{{ i + 1 }}</span>
            <span class="font-mono text-[11px] text-text-muted">ID: {{ item.id }}</span>
            <span class="font-mono text-[11px] text-green-500">score: {{ item.score?.toFixed(4) ?? '-' }}</span>
            <span v-if="item.expanded_from" class="flex items-center gap-1 text-[10px] text-yellow-500 px-1.5 py-0.5 bg-yellow-950/30 border border-yellow-800/20 rounded">
              <font-awesome-icon icon="arrow-up-right-from-square" class="w-2.5" />
              from #{{ item.expanded_from }}
            </span>
          </div>
          <div v-if="item.metadata" class="flex flex-wrap gap-1 mb-1.5">
            <span v-for="(v, k) in item.metadata" :key="k" class="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] text-text-muted font-mono">
              {{ k }}={{ v }}
            </span>
          </div>
          <div v-if="item.memory_type" class="mt-1">
            <span
              class="px-1.5 py-0.5 rounded text-[10px] font-medium uppercase tracking-wide"
              :class="item.memory_type === 'Episodic' ? 'bg-purple-950/60 text-purple-400 border border-purple-800/30' : 'bg-orange-950/60 text-orange-400 border border-orange-800/30'"
            >
              {{ item.memory_type }}
            </span>
          </div>
        </div>
      </div>

      <div class="w-[260px] flex-shrink-0 flex flex-col gap-4">
        <div class="bg-bg-secondary border border-border-color rounded-xl p-4">
          <div class="flex items-center gap-2 mb-3">
            <font-awesome-icon icon="clipboard" class="w-3 text-text-muted" />
            <h3 class="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">Examples</h3>
          </div>
          <button
            v-for="ex in examples"
            :key="ex.query"
            class="block w-full text-left px-3 py-2 mb-1.5 bg-bg-primary border border-border-color rounded-lg cursor-pointer transition-all duration-150 hover:border-border-hover hover:bg-bg-tertiary"
            @click="loadExample(ex.query)"
          >
            <span class="block text-[11px] font-medium text-text-primary mb-0.5">{{ ex.label }}</span>
            <code class="text-[10px] text-text-muted font-mono">{{ ex.query }}</code>
          </button>
        </div>

        <div v-if="queryHistory.length > 0" class="bg-bg-secondary border border-border-color rounded-xl p-4">
          <div class="flex justify-between items-center mb-3">
            <div class="flex items-center gap-2">
              <font-awesome-icon icon="history" class="w-3 text-text-muted" />
              <h3 class="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">History</h3>
            </div>
            <button
              class="flex items-center gap-1 text-[10px] text-text-muted bg-transparent border-none cursor-pointer hover:text-red-400 transition-colors"
              @click="clearHistory"
            >
              <font-awesome-icon icon="broom" class="w-2.5" />
              Clear
            </button>
          </div>
          <button
            v-for="(q, i) in queryHistory"
            :key="i"
            class="block w-full text-left px-3 py-1.5 mb-1 bg-transparent border border-border-color rounded-md cursor-pointer transition-all duration-150 hover:border-border-hover hover:bg-bg-tertiary"
            @click="loadFromHistory(q)"
          >
            <code class="text-[10px] text-text-muted font-mono">{{ q }}</code>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
