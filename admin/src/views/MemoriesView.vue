<script setup lang="ts">
import { ref } from 'vue'
import { useApi } from '../composables/useApi'
import type { MemoryItem } from '../composables/useApi'

const api = useApi()
const results = ref<MemoryItem[]>([])
const selectedId = ref<number | null>(null)
const detailItem = ref<MemoryItem | null>(null)

const searchVector = ref('')
const searchK = ref(10)
const searchMetric = ref('cs')

const showInsertForm = ref(false)
const insertVector = ref('')
const insertMeta = ref('')
const insertType = ref('semantic')

const loading = ref(false)
const error = ref<string | null>(null)

async function doSearch() {
  loading.value = true
  error.value = null
  try {
    const vector = searchVector.value.split(',').map(Number).filter(n => !isNaN(n))
    if (vector.length === 0) {
      error.value = '请输入有效的向量，用逗号分隔'
      return
    }
    results.value = await api.search({
      vector,
      k: searchK.value,
      metric: searchMetric.value,
    })
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function viewDetail(item: MemoryItem) {
  selectedId.value = item.id
  try {
    detailItem.value = await api.getMemory(item.id)
    await api.accessMemory(item.id)
  } catch (e: any) {
    error.value = e.message
  }
}

async function deleteItem(id: number) {
  if (!confirm('确定删除该记忆？')) return
  try {
    await api.deleteMemory(id)
    results.value = results.value.filter(r => r.id !== id)
    if (detailItem.value?.id === id) {
      detailItem.value = null
      selectedId.value = null
    }
  } catch (e: any) {
    error.value = e.message
  }
}

async function doInsert() {
  loading.value = true
  error.value = null
  try {
    const vector = insertVector.value.split(',').map(Number).filter(n => !isNaN(n))
    if (vector.length === 0) {
      error.value = '请输入有效的向量'
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
    await api.insertMemory({ vector, metadata, memory_type: insertType.value })
    showInsertForm.value = false
    insertVector.value = ''
    insertMeta.value = ''
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
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
        <input
          v-model="searchVector"
          placeholder="向量 (例: 0.1,0.2,0.3,...)"
          class="flex-1 min-w-[200px] bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
        />
        <select
          v-model="searchMetric"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option value="cs">余弦 (Cosine)</option>
          <option value="eu">欧氏 (Euclidean)</option>
          <option value="dot">点积 (Dot Product)</option>
        </select>
        <input
          v-model.number="searchK"
          type="number"
          min="1"
          max="100"
          class="w-[70px] bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
          placeholder="K"
        />
        <button
          class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doSearch"
        >
          {{ loading ? '搜索中...' : '搜索' }}
        </button>
        <button
          class="px-4 py-2 border border-border-color rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-bg-tertiary text-text-primary hover:bg-bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
          @click="showInsertForm = !showInsertForm"
        >
          {{ showInsertForm ? '取消' : '+ 插入记忆' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="px-4 py-2.5 bg-red-900/50 border border-red-600 rounded-lg text-red-400 text-sm">{{ error }}</div>

    <div v-if="showInsertForm" class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-2.5">
      <h3 class="m-0 mb-1 text-base">插入新记忆</h3>
      <input
        v-model="insertVector"
        placeholder="向量 (逗号分隔)"
        class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
      />
      <textarea
        v-model="insertMeta"
        placeholder="元数据 (每行 key=value)"
        class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary resize-y font-mono"
        rows="3"
      ></textarea>
      <select
        v-model="insertType"
        class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
      >
        <option value="semantic">语义记忆</option>
        <option value="episodic">情景记忆</option>
      </select>
      <button
        class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="loading"
        @click="doInsert"
      >
        提交
      </button>
    </div>

    <div class="flex gap-4 min-h-[400px]">
      <div class="flex-1 flex flex-col gap-2 overflow-auto">
        <div v-if="results.length === 0 && !loading" class="text-text-secondary text-center py-16 text-sm">
          输入向量开始搜索记忆
        </div>
        <div
          v-for="item in results"
          :key="item.id"
          class="bg-bg-secondary border border-border-color rounded-lg p-3 cursor-pointer transition-all duration-150 hover:border-accent-primary"
          :class="{ 'border-accent-primary bg-bg-tertiary': selectedId === item.id }"
          @click="viewDetail(item)"
        >
          <div class="flex items-center gap-2 mb-1.5">
            <span class="font-semibold text-sm text-accent-primary">#{{ item.id }}</span>
            <span
              class="px-2 py-0.5 rounded-full text-xs font-medium"
              :class="item.memory_type === 'Episodic' ? 'bg-purple-900/50 text-purple-400' : 'bg-green-900/50 text-green-400'"
            >
              {{ item.memory_type }}
            </span>
            <button
              class="ml-auto bg-transparent border-none cursor-pointer text-sm p-0.5 rounded text-text-secondary transition-all duration-150 hover:bg-red-600 hover:text-white"
              @click.stop="deleteItem(item.id)"
              title="删除"
            >
              ✕
            </button>
          </div>
          <div class="flex gap-3 text-xs text-text-secondary mb-1.5">
            <span>Score: {{ item.score.toFixed(4) }}</span>
            <span>Dist: {{ item.distance.toFixed(4) }}</span>
          </div>
          <div class="flex flex-wrap gap-1 mb-1.5">
            <span v-for="(v, k) in item.metadata" :key="k" class="px-1.5 py-0.5 bg-bg-tertiary rounded text-xs text-text-secondary">
              {{ k }}={{ v }}
            </span>
          </div>
          <div class="flex gap-3 text-xs text-text-secondary/70">
            <span>衰减: {{ item.decay_score.toFixed(3) }}</span>
            <span>访问: {{ item.access_count }}</span>
          </div>
        </div>
      </div>

      <div v-if="detailItem" class="w-[340px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl p-5 overflow-auto">
        <h3 class="m-0 mb-4 text-base">记忆详情 #{{ detailItem.id }}</h3>
        <div class="mb-4">
          <div class="text-xs text-text-secondary uppercase tracking-wider mb-1.5">向量</div>
          <div class="font-mono text-xs text-text-secondary bg-bg-primary p-2 rounded break-all">
            [{{ detailItem.vector.slice(0, 8).map(v => v.toFixed(4)).join(', ') }}{{ detailItem.vector.length > 8 ? ', ...' : '' }}]
          </div>
        </div>
        <div class="mb-4">
          <div class="text-xs text-text-secondary uppercase tracking-wider mb-1.5">元数据</div>
          <div v-for="(v, k) in detailItem.metadata" :key="k" class="flex justify-between py-1 text-sm">
            <span class="text-text-secondary">{{ k }}</span>
            <span class="text-text-primary font-mono text-xs break-all max-w-[200px] text-right">{{ v }}</span>
          </div>
        </div>
        <div>
          <div class="text-xs text-text-secondary uppercase tracking-wider mb-1.5">时间信息</div>
          <div class="flex justify-between py-1 text-sm">
            <span class="text-text-secondary">创建时间</span>
            <span class="text-text-primary font-mono text-xs">{{ formatTimestamp(detailItem.created_at) }}</span>
          </div>
          <div class="flex justify-between py-1 text-sm">
            <span class="text-text-secondary">访问次数</span>
            <span class="text-text-primary font-mono text-xs">{{ detailItem.access_count }}</span>
          </div>
          <div class="flex justify-between py-1 text-sm">
            <span class="text-text-secondary">衰减分数</span>
            <span class="text-text-primary font-mono text-xs">{{ detailItem.decay_score.toFixed(4) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
