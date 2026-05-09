<script setup lang="ts">
import { ref } from 'vue'
import { useApi } from '../composables/useApi'

const api = useApi()

const queryText = ref('BOOST recent')
const queryK = ref(10)
const queryMetric = ref('cs')
const results = ref<any[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const queryHistory = ref<string[]>([])

const examples = [
  { label: '最近记忆', query: 'BOOST recent' },
  { label: '语义搜索', query: 'VECTOR_SIMILAR("text")' },
  { label: '属性过滤', query: 'category = technology' },
  { label: '组合查询', query: 'VECTOR_SIMILAR("memory") AND category = important BOOST recent 2.0' },
  { label: '图扩展', query: 'EXPAND depth=2' },
  { label: '或查询', query: 'VECTOR_SIMILAR("test") OR category = demo' },
]

async function doQuery() {
  if (!queryText.value.trim()) {
    error.value = '请输入查询语句'
    return
  }
  loading.value = true
  error.value = null
  try {
    results.value = await api.queryExec({
      query: queryText.value,
      k: queryK.value,
      metric: queryMetric.value,
    })
    if (!queryHistory.value.includes(queryText.value)) {
      queryHistory.value.unshift(queryText.value)
      if (queryHistory.value.length > 20) {
        queryHistory.value.pop()
      }
    }
  } catch (e: any) {
    error.value = e.message
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
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl overflow-hidden">
      <div class="flex justify-between items-center px-4 py-3 border-b border-border-color">
        <h3 class="m-0 text-base">语义查询</h3>
        <div class="flex gap-1.5 flex-wrap">
          <span class="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-secondary">VECTOR_SIMILAR("text")</span>
          <span class="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-secondary">key = value</span>
          <span class="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-secondary">AND / OR</span>
          <span class="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-secondary">BOOST recent</span>
          <span class="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-secondary">EXPAND depth=N</span>
        </div>
      </div>
      <div class="px-4 py-3">
        <textarea
          v-model="queryText"
          placeholder="输入查询语句..."
          class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-3 text-text-primary text-sm font-mono resize-y outline-none leading-6 transition-all duration-150 focus:border-accent-primary"
          rows="4"
          @keydown.ctrl.enter="doQuery"
        ></textarea>
      </div>
      <div class="flex justify-between items-center px-4 py-3 border-t border-border-color">
        <div class="flex gap-2 items-center">
          <select
            v-model="queryMetric"
            class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
          >
            <option value="cs">余弦 (Cosine)</option>
            <option value="eu">欧氏 (Euclidean)</option>
            <option value="dot">点积 (Dot Product)</option>
          </select>
          <input
            v-model.number="queryK"
            type="number"
            min="1"
            max="100"
            class="w-[70px] bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
            placeholder="K"
          />
        </div>
        <button
          class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doQuery"
        >
          {{ loading ? '执行中...' : '执行查询 (Ctrl+Enter)' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="px-4 py-2.5 bg-red-900/50 border border-red-600 rounded-lg text-red-400 text-sm">{{ error }}</div>

    <div class="flex gap-4 min-h-[400px]">
      <div class="flex-1 flex flex-col gap-2">
        <h3 class="m-0 mb-2 text-base">结果 ({{ results.length }})</h3>
        <div v-if="results.length === 0 && !loading" class="text-text-secondary text-center py-16 text-sm">
          执行查询查看结果
        </div>
        <div v-for="(item, i) in results" :key="i" class="bg-bg-secondary border border-border-color rounded-lg px-4 py-3 transition-all duration-150 hover:border-accent-primary">
          <div class="flex items-center gap-2.5 mb-1.5">
            <span class="font-bold text-accent-primary text-sm">#{{ i + 1 }}</span>
            <span class="font-mono text-xs text-text-secondary">ID: {{ item.id }}</span>
            <span class="font-mono text-xs text-green-400">Score: {{ item.score?.toFixed(4) ?? '-' }}</span>
            <span v-if="item.expanded_from" class="text-xs text-yellow-500 px-1 py-0.5 bg-yellow-900/30 rounded">
              Expanded from #{{ item.expanded_from }}
            </span>
          </div>
          <div v-if="item.metadata" class="flex flex-wrap gap-1 mb-1">
            <span v-for="(v, k) in item.metadata" :key="k" class="px-1.5 py-0.5 bg-bg-tertiary rounded text-xs text-text-secondary">
              {{ k }}={{ v }}
            </span>
          </div>
          <div v-if="item.memory_type" class="mt-1">
            <span
              class="px-2 py-0.5 rounded-full text-xs font-medium"
              :class="item.memory_type === 'Episodic' ? 'bg-purple-900/50 text-purple-400' : 'bg-green-900/50 text-green-400'"
            >
              {{ item.memory_type }}
            </span>
          </div>
        </div>
      </div>

      <div class="w-[260px] flex-shrink-0 flex flex-col gap-4">
        <div class="bg-bg-secondary border border-border-color rounded-xl p-4">
          <h3 class="m-0 mb-2.5 text-sm">示例查询</h3>
          <button
            v-for="ex in examples"
            :key="ex.query"
            class="block w-full text-left px-2.5 py-2 mb-1.5 bg-bg-primary border border-bg-tertiary rounded-lg cursor-pointer transition-all duration-150 hover:border-accent-primary hover:bg-bg-tertiary"
            @click="loadExample(ex.query)"
          >
            <span class="block text-xs text-text-primary mb-0.5">{{ ex.label }}</span>
            <code class="text-xs text-text-secondary font-mono">{{ ex.query }}</code>
          </button>
        </div>

        <div v-if="queryHistory.length > 0" class="bg-bg-secondary border border-border-color rounded-xl p-4">
          <h3 class="m-0 mb-2.5 text-sm">查询历史</h3>
          <button
            v-for="(q, i) in queryHistory"
            :key="i"
            class="block w-full text-left px-2.5 py-1.5 mb-1 bg-transparent border border-bg-tertiary rounded-md cursor-pointer transition-all duration-150 hover:border-accent-primary hover:bg-bg-tertiary"
            @click="loadFromHistory(q)"
          >
            <code class="text-xs text-text-secondary">{{ q }}</code>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
