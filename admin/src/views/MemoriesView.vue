<script setup lang="ts">
import { ref, onMounted } from 'vue'
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
  <div class="memories-view">
    <div class="toolbar">
      <div class="search-bar">
        <input
          v-model="searchVector"
          placeholder="向量 (例: 0.1,0.2,0.3,...)"
          class="input-field wide"
        />
        <select v-model="searchMetric" class="input-field">
          <option value="cs">余弦 (Cosine)</option>
          <option value="eu">欧氏 (Euclidean)</option>
          <option value="dot">点积 (Dot Product)</option>
        </select>
        <input v-model.number="searchK" type="number" min="1" max="100" class="input-field narrow" placeholder="K" />
        <button class="btn btn-primary" :disabled="loading" @click="doSearch">
          {{ loading ? '搜索中...' : '搜索' }}
        </button>
        <button class="btn btn-secondary" @click="showInsertForm = !showInsertForm">
          {{ showInsertForm ? '取消' : '+ 插入记忆' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="error-bar">{{ error }}</div>

    <div v-if="showInsertForm" class="insert-form">
      <h3>插入新记忆</h3>
      <input v-model="insertVector" placeholder="向量 (逗号分隔)" class="input-field full" />
      <textarea v-model="insertMeta" placeholder="元数据 (每行 key=value)" class="input-field full" rows="3"></textarea>
      <select v-model="insertType" class="input-field">
        <option value="semantic">语义记忆</option>
        <option value="episodic">情景记忆</option>
      </select>
      <button class="btn btn-primary" :disabled="loading" @click="doInsert">提交</button>
    </div>

    <div class="content-area">
      <div class="list-panel">
        <div v-if="results.length === 0 && !loading" class="empty-hint">
          输入向量开始搜索记忆
        </div>
        <div
          v-for="item in results"
          :key="item.id"
          :class="['memory-card', { selected: selectedId === item.id }]"
          @click="viewDetail(item)"
        >
          <div class="card-header">
            <span class="card-id">#{{ item.id }}</span>
            <span :class="['type-tag', item.memory_type === 'Episodic' ? 'episodic' : 'semantic']">
              {{ item.memory_type }}
            </span>
            <button class="btn-icon danger" @click.stop="deleteItem(item.id)" title="删除">✕</button>
          </div>
          <div class="card-score">
            <span>Score: {{ item.score.toFixed(4) }}</span>
            <span>Dist: {{ item.distance.toFixed(4) }}</span>
          </div>
          <div class="card-meta">
            <span v-for="(v, k) in item.metadata" :key="k" class="meta-chip">
              {{ k }}={{ v }}
            </span>
          </div>
          <div class="card-temporal">
            <span>衰减: {{ item.decay_score.toFixed(3) }}</span>
            <span>访问: {{ item.access_count }}</span>
          </div>
        </div>
      </div>

      <div v-if="detailItem" class="detail-panel">
        <h3>记忆详情 #{{ detailItem.id }}</h3>
        <div class="detail-section">
          <div class="detail-label">向量</div>
          <div class="vector-preview">
            [{{ detailItem.vector.slice(0, 8).map(v => v.toFixed(4)).join(', ') }}{{ detailItem.vector.length > 8 ? ', ...' : '' }}]
          </div>
        </div>
        <div class="detail-section">
          <div class="detail-label">元数据</div>
          <div v-for="(v, k) in detailItem.metadata" :key="k" class="detail-row">
            <span class="detail-key">{{ k }}</span>
            <span class="detail-val">{{ v }}</span>
          </div>
        </div>
        <div class="detail-section">
          <div class="detail-label">时间信息</div>
          <div class="detail-row">
            <span class="detail-key">创建时间</span>
            <span class="detail-val">{{ formatTimestamp(detailItem.created_at) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-key">访问次数</span>
            <span class="detail-val">{{ detailItem.access_count }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-key">衰减分数</span>
            <span class="detail-val">{{ detailItem.decay_score.toFixed(4) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.memories-view {
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

.search-bar {
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
  transition: border-color 0.15s;
}

.input-field:focus {
  border-color: #58a6ff;
}

.input-field.wide { flex: 1; min-width: 200px; }
.input-field.narrow { width: 70px; }
.input-field.full { width: 100%; }

textarea.input-field {
  resize: vertical;
  font-family: monospace;
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

.btn-secondary:hover {
  background: #30363d;
}

.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 4px;
  color: #8b949e;
}

.btn-icon.danger:hover {
  background: #da3633;
  color: #fff;
}

.error-bar {
  padding: 10px 16px;
  background: #3d1414;
  border: 1px solid #da3633;
  border-radius: 8px;
  color: #f85149;
  font-size: 13px;
}

.insert-form {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.insert-form h3 {
  margin: 0 0 4px 0;
  font-size: 15px;
}

.content-area {
  display: flex;
  gap: 16px;
  min-height: 400px;
}

.list-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: auto;
}

.detail-panel {
  width: 340px;
  flex-shrink: 0;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
  overflow: auto;
}

.detail-panel h3 {
  margin: 0 0 16px 0;
  font-size: 15px;
}

.detail-section {
  margin-bottom: 16px;
}

.detail-label {
  font-size: 12px;
  color: #8b949e;
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 13px;
}

.detail-key {
  color: #8b949e;
}

.detail-val {
  color: #e1e4e8;
  font-family: monospace;
  font-size: 12px;
  word-break: break-all;
  max-width: 200px;
  text-align: right;
}

.vector-preview {
  font-family: monospace;
  font-size: 12px;
  color: #8b949e;
  background: #0d1117;
  padding: 8px;
  border-radius: 6px;
  word-break: break-all;
}

.memory-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  padding: 12px 16px;
  cursor: pointer;
  transition: all 0.15s;
}

.memory-card:hover {
  border-color: #58a6ff;
}

.memory-card.selected {
  border-color: #58a6ff;
  background: #1c2333;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.card-id {
  font-weight: 600;
  font-size: 14px;
  color: #58a6ff;
}

.type-tag {
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
}

.type-tag.semantic {
  background: #1c3a2a;
  color: #3fb950;
}

.type-tag.episodic {
  background: #2d1f5e;
  color: #bc8cff;
}

.card-score {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: #8b949e;
  margin-bottom: 6px;
}

.card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 6px;
}

.meta-chip {
  padding: 2px 6px;
  background: #1c2333;
  border-radius: 4px;
  font-size: 11px;
  color: #8b949e;
}

.card-temporal {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: #6e7681;
}

.empty-hint {
  color: #6e7681;
  text-align: center;
  padding: 60px 0;
  font-size: 14px;
}
</style>
