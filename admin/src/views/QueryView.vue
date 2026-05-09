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
  <div class="query-view">
    <div class="query-editor">
      <div class="editor-header">
        <h3>语义查询</h3>
        <div class="editor-hints">
          <span class="hint-chip">VECTOR_SIMILAR("text")</span>
          <span class="hint-chip">key = value</span>
          <span class="hint-chip">AND / OR</span>
          <span class="hint-chip">BOOST recent</span>
          <span class="hint-chip">EXPAND depth=N</span>
        </div>
      </div>
      <div class="editor-body">
        <textarea
          v-model="queryText"
          placeholder="输入查询语句..."
          class="query-input"
          rows="4"
          @keydown.ctrl.enter="doQuery"
        ></textarea>
      </div>
      <div class="editor-footer">
        <div class="editor-options">
          <select v-model="queryMetric" class="input-field">
            <option value="cs">余弦 (Cosine)</option>
            <option value="eu">欧氏 (Euclidean)</option>
            <option value="dot">点积 (Dot Product)</option>
          </select>
          <input v-model.number="queryK" type="number" min="1" max="100" class="input-field narrow" placeholder="K" />
        </div>
        <button class="btn btn-primary" :disabled="loading" @click="doQuery">
          {{ loading ? '执行中...' : '执行查询 (Ctrl+Enter)' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="alert error">{{ error }}</div>

    <div class="query-layout">
      <div class="results-panel">
        <h3>结果 ({{ results.length }})</h3>
        <div v-if="results.length === 0 && !loading" class="empty-hint">
          执行查询查看结果
        </div>
        <div v-for="(item, i) in results" :key="i" class="result-card">
          <div class="result-header">
            <span class="result-rank">#{{ i + 1 }}</span>
            <span class="result-id">ID: {{ item.id }}</span>
            <span class="result-score">Score: {{ item.score?.toFixed(4) ?? '-' }}</span>
            <span v-if="item.expanded_from" class="result-expanded">
              Expanded from #{{ item.expanded_from }}
            </span>
          </div>
          <div v-if="item.metadata" class="result-meta">
            <span v-for="(v, k) in item.metadata" :key="k" class="meta-chip">
              {{ k }}={{ v }}
            </span>
          </div>
          <div v-if="item.memory_type" class="result-type">
            <span :class="['type-tag', item.memory_type === 'Episodic' ? 'episodic' : 'semantic']">
              {{ item.memory_type }}
            </span>
          </div>
        </div>
      </div>

      <div class="side-panel">
        <div class="examples-section">
          <h3>示例查询</h3>
          <button
            v-for="ex in examples"
            :key="ex.query"
            class="example-btn"
            @click="loadExample(ex.query)"
          >
            <span class="example-label">{{ ex.label }}</span>
            <code class="example-code">{{ ex.query }}</code>
          </button>
        </div>

        <div v-if="queryHistory.length > 0" class="history-section">
          <h3>查询历史</h3>
          <button
            v-for="(q, i) in queryHistory"
            :key="i"
            class="history-btn"
            @click="loadFromHistory(q)"
          >
            <code>{{ q }}</code>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.query-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.query-editor {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow: hidden;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #30363d;
}

.editor-header h3 {
  margin: 0;
  font-size: 15px;
}

.editor-hints {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.hint-chip {
  padding: 2px 8px;
  background: #1c2333;
  border-radius: 4px;
  font-size: 11px;
  font-family: monospace;
  color: #8b949e;
}

.editor-body {
  padding: 12px 16px;
}

.query-input {
  width: 100%;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 12px;
  color: #e1e4e8;
  font-size: 14px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  resize: vertical;
  outline: none;
  line-height: 1.6;
}

.query-input:focus {
  border-color: #58a6ff;
}

.editor-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-top: 1px solid #30363d;
}

.editor-options {
  display: flex;
  gap: 8px;
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
  width: 70px;
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

.alert.error {
  padding: 10px 16px;
  background: #3d1414;
  border: 1px solid #da3633;
  border-radius: 8px;
  color: #f85149;
  font-size: 13px;
}

.query-layout {
  display: flex;
  gap: 16px;
  min-height: 400px;
}

.results-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.results-panel h3 {
  margin: 0 0 8px 0;
  font-size: 15px;
}

.result-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  padding: 12px 16px;
  transition: border-color 0.15s;
}

.result-card:hover {
  border-color: #58a6ff;
}

.result-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.result-rank {
  font-weight: 700;
  color: #58a6ff;
  font-size: 14px;
}

.result-id {
  font-family: monospace;
  font-size: 12px;
  color: #8b949e;
}

.result-score {
  font-family: monospace;
  font-size: 12px;
  color: #3fb950;
}

.result-expanded {
  font-size: 11px;
  color: #d29922;
  padding: 1px 6px;
  background: #2d1f00;
  border-radius: 4px;
}

.result-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.meta-chip {
  padding: 2px 6px;
  background: #1c2333;
  border-radius: 4px;
  font-size: 11px;
  color: #8b949e;
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

.result-type {
  margin-top: 4px;
}

.side-panel {
  width: 260px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.examples-section,
.history-section {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 16px;
}

.examples-section h3,
.history-section h3 {
  margin: 0 0 10px 0;
  font-size: 14px;
}

.example-btn {
  display: block;
  width: 100%;
  text-align: left;
  padding: 8px 10px;
  margin-bottom: 6px;
  background: #0d1117;
  border: 1px solid #21262d;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.example-btn:hover {
  border-color: #58a6ff;
  background: #1c2333;
}

.example-label {
  display: block;
  font-size: 12px;
  color: #e1e4e8;
  margin-bottom: 2px;
}

.example-code {
  font-size: 11px;
  color: #8b949e;
  font-family: monospace;
}

.history-btn {
  display: block;
  width: 100%;
  text-align: left;
  padding: 6px 10px;
  margin-bottom: 4px;
  background: transparent;
  border: 1px solid #21262d;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.history-btn:hover {
  border-color: #58a6ff;
  background: #1c2333;
}

.history-btn code {
  font-size: 11px;
  color: #8b949e;
}

.empty-hint {
  color: #6e7681;
  text-align: center;
  padding: 60px 0;
  font-size: 14px;
}
</style>
