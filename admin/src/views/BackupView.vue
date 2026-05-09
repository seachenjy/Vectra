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
  <div class="flex flex-col gap-5">
    <div v-if="error" class="px-4 py-2.5 bg-red-900/50 border border-red-600 rounded-lg text-red-400 text-sm">{{ error }}</div>
    <div v-if="success" class="px-4 py-2.5 bg-green-900/50 border border-green-600 rounded-lg text-green-400 text-sm">{{ success }}</div>

    <div class="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-1.5 text-base">创建备份</h3>
        <p class="text-text-secondary text-sm m-0 mb-3">将当前所有记忆节点保存为快照</p>
        <button
          class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="createBackup"
        >
          {{ loading ? '处理中...' : '创建快照' }}
        </button>
      </div>

      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-1.5 text-base">导出数据</h3>
        <p class="text-text-secondary text-sm m-0 mb-3">将所有记忆导出为 SkyArchive JSON 文件</p>
        <button
          class="px-4 py-2 border border-border-color rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-bg-tertiary text-text-primary hover:bg-bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="doExport"
        >
          下载导出文件
        </button>
      </div>

      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-1.5 text-base">导入数据</h3>
        <p class="text-text-secondary text-sm m-0 mb-3">从文件导入记忆节点</p>
        <div class="flex flex-col gap-2">
          <input
            v-model="importPath"
            placeholder="文件路径"
            class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
          />
          <select
            v-model="importFormat"
            class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary"
          >
            <option value="jsonl">JSONL</option>
            <option value="csv">CSV</option>
            <option value="archive">SkyArchive</option>
          </select>
          <button
            class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="loading"
            @click="doImport"
          >
            导入
          </button>
        </div>
      </div>
    </div>

    <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
      <h3 class="m-0 mb-3 text-base">历史备份 ({{ backups.length }})</h3>
      <div v-if="backups.length === 0" class="text-text-secondary text-sm text-center py-6">暂无备份</div>
      <div v-for="snap in backups" :key="snap" class="flex justify-between items-center py-2.5 border-b border-bg-tertiary">
        <div class="flex flex-col gap-0.5">
          <span class="font-mono text-sm text-accent-primary">{{ snap }}</span>
          <span class="text-xs text-text-secondary">{{ formatSnapshotId(snap) }}</span>
        </div>
        <button
          class="px-2.5 py-1 border border-border-color rounded-lg text-xs cursor-pointer font-medium transition-all duration-150 bg-bg-tertiary text-text-primary hover:bg-bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="loading"
          @click="restoreBackup(snap)"
        >
          恢复
        </button>
      </div>
    </div>
  </div>
</template>
