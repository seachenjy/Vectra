<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'

const api = useApi()
const backups = ref<string[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const success = ref<string | null>(null)

const importPath = ref('')
const importFormat = ref('jsonl')

onMounted(async () => {
  await loadBackups()
})

async function loadBackups() {
  try {
    backups.value = await api.listBackups()
  } catch (e: any) {
    error.value = e.message
  }
}

async function createBackup() {
  loading.value = true
  error.value = null
  success.value = null
  try {
    const result = await api.handleBackup('create')
    if (result.ok) {
      success.value = `备份已创建: ${result.snapshot_id}`
      await loadBackups()
    } else {
      error.value = result.error
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function restoreBackup(snapshotId: string) {
  if (!confirm(`确定恢复备份 ${snapshotId}？当前数据将被覆盖。`)) return
  loading.value = true
  error.value = null
  success.value = null
  try {
    const result = await api.handleBackup('restore', snapshotId)
    if (result.ok) {
      success.value = '备份已恢复'
    } else {
      error.value = result.error
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function doImport() {
  if (!importPath.value.trim()) {
    error.value = '请输入文件路径'
    return
  }
  loading.value = true
  error.value = null
  success.value = null
  try {
    const result = await api.handleImport({ format: importFormat.value, path: importPath.value })
    if (result.ok) {
      success.value = `导入成功: ${result.imported} 条记忆`
    } else {
      error.value = result.error
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function doExport() {
  loading.value = true
  error.value = null
  success.value = null
  try {
    const blob = await api.handleExport()
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'skymemory_export.json'
    a.click()
    URL.revokeObjectURL(url)
    success.value = '导出完成'
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function formatSnapshotId(id: string): string {
  const ts = id.replace('snap_', '')
  const num = parseInt(ts)
  if (!isNaN(num)) {
    return new Date(num).toLocaleString('zh-CN')
  }
  return id
}
</script>

<template>
  <div class="backup-view">
    <div v-if="error" class="alert error">{{ error }}</div>
    <div v-if="success" class="alert success">{{ success }}</div>

    <div class="backup-grid">
      <div class="backup-card">
        <h3>创建备份</h3>
        <p class="card-desc">将当前所有记忆节点保存为快照</p>
        <button class="btn btn-primary" :disabled="loading" @click="createBackup">
          {{ loading ? '处理中...' : '创建快照' }}
        </button>
      </div>

      <div class="backup-card">
        <h3>导出数据</h3>
        <p class="card-desc">将所有记忆导出为 SkyArchive JSON 文件</p>
        <button class="btn btn-secondary" :disabled="loading" @click="doExport">
          下载导出文件
        </button>
      </div>

      <div class="backup-card">
        <h3>导入数据</h3>
        <p class="card-desc">从文件导入记忆节点</p>
        <div class="import-form">
          <input v-model="importPath" placeholder="文件路径" class="input-field full" />
          <select v-model="importFormat" class="input-field">
            <option value="jsonl">JSONL</option>
            <option value="csv">CSV</option>
            <option value="archive">SkyArchive</option>
          </select>
          <button class="btn btn-primary" :disabled="loading" @click="doImport">导入</button>
        </div>
      </div>
    </div>

    <div class="snapshots-section">
      <h3>历史备份 ({{ backups.length }})</h3>
      <div v-if="backups.length === 0" class="empty-hint">暂无备份</div>
      <div v-for="snap in backups" :key="snap" class="snapshot-row">
        <div class="snapshot-info">
          <span class="snapshot-id">{{ snap }}</span>
          <span class="snapshot-time">{{ formatSnapshotId(snap) }}</span>
        </div>
        <button class="btn btn-secondary btn-sm" :disabled="loading" @click="restoreBackup(snap)">
          恢复
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backup-view {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.alert {
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 13px;
}

.alert.error {
  background: #3d1414;
  border: 1px solid #da3633;
  color: #f85149;
}

.alert.success {
  background: #1c3a2a;
  border: 1px solid #238636;
  color: #3fb950;
}

.backup-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}

.backup-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
}

.backup-card h3 {
  margin: 0 0 6px 0;
  font-size: 15px;
}

.card-desc {
  color: #8b949e;
  font-size: 13px;
  margin: 0 0 12px 0;
}

.import-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
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

.input-field.full {
  width: 100%;
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

.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
}

.snapshots-section {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
}

.snapshots-section h3 {
  margin: 0 0 12px 0;
  font-size: 15px;
}

.snapshot-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 0;
  border-bottom: 1px solid #21262d;
}

.snapshot-row:last-child {
  border-bottom: none;
}

.snapshot-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.snapshot-id {
  font-family: monospace;
  font-size: 13px;
  color: #58a6ff;
}

.snapshot-time {
  font-size: 12px;
  color: #6e7681;
}

.empty-hint {
  color: #6e7681;
  text-align: center;
  padding: 24px 0;
  font-size: 13px;
}
</style>
