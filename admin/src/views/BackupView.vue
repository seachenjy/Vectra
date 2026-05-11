<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'

const api = useApi()
const toast = useToast()
const store = useAppStore()
const backups = ref<string[]>([])

const creating = ref(false)
const restoring = ref(new Set<string>())
const importing = ref(false)
const exporting = ref(false)

const importPath = ref('')
const importFormat = ref('jsonl')

onMounted(async () => {
  await loadBackups()
})

watch(() => store.currentNs, () => {
  loadBackups()
})

async function loadBackups() {
  try {
    backups.value = await api.listBackups(store.currentNs)
  } catch (e: any) {
    toast.error(e.message)
  }
}

async function createBackup() {
  creating.value = true
  try {
    const result = await api.handleBackup(store.currentNs, 'create')
    if (result.ok) {
      toast.success(`Backup created: ${result.snapshot_id}`)
      await loadBackups()
    } else {
      toast.error(result.error)
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    creating.value = false
  }
}

async function restoreBackup(snapshotId: string) {
  if (!confirm(`Restore backup ${snapshotId}? Current data will be overwritten.`)) return
  restoring.value.add(snapshotId)
  try {
    const result = await api.handleBackup(store.currentNs, 'restore', snapshotId)
    if (result.ok) {
      toast.success('Backup restored')
      await Promise.all([loadBackups(), store.fetchInfo(), store.fetchMetrics()])
    } else {
      toast.error(result.error)
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    restoring.value.delete(snapshotId)
  }
}

async function doImport() {
  if (!importPath.value.trim()) {
    toast.error('Enter a file path')
    return
  }
  importing.value = true
  try {
    const result = await api.handleImport(store.currentNs, { format: importFormat.value, path: importPath.value })
    if (result.ok) {
      toast.success(`Imported ${result.imported} memories`)
    } else {
      toast.error(result.error)
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    importing.value = false
  }
}

async function doExport() {
  exporting.value = true
  try {
    const blob = await api.handleExport(store.currentNs)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
    a.download = `skymemory_export_${ts}.json`
    a.click()
    URL.revokeObjectURL(url)
    toast.success('Export complete')
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    exporting.value = false
  }
}

function formatSnapshotId(id: string): string {
  const ts = id.replace('snap_', '')
  const num = parseInt(ts)
  if (!isNaN(num)) return new Date(num).toLocaleString('zh-CN')
  return id
}

const actions = [
  {
    key: 'backup',
    title: 'Create Backup',
    desc: 'Save all memory nodes as a snapshot',
    icon: 'floppy-disk',
    btnText: 'Create Snapshot',
    btnStyle: 'primary',
    loading: creating,
    handler: createBackup,
  },
  {
    key: 'export',
    title: 'Export Data',
    desc: 'Export all memories as SkyArchive JSON',
    icon: 'file-export',
    btnText: 'Download Export',
    btnStyle: 'secondary',
    loading: exporting,
    handler: doExport,
  },
]
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
      <div
        v-for="action in actions"
        :key="action.key"
        class="bg-bg-secondary border border-border-color rounded-xl p-5"
      >
        <div class="flex items-center gap-2 mb-1.5">
          <font-awesome-icon :icon="action.icon" class="w-3.5 text-accent-primary" />
          <h3 class="m-0 text-sm font-semibold">{{ action.title }}</h3>
        </div>
        <p class="text-text-muted text-xs m-0 mb-4">{{ action.desc }}</p>
        <button
          class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed"
          :class="action.btnStyle === 'primary'
            ? 'bg-accent-primary text-white hover:brightness-110'
            : 'bg-bg-tertiary border border-border-color text-text-secondary hover:border-border-hover hover:text-text-primary'"
          :disabled="action.loading.value"
          @click="action.handler"
        >
          <font-awesome-icon v-if="action.loading.value" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else :icon="action.icon" class="w-3.5" />
          {{ action.loading.value ? 'Processing...' : action.btnText }}
        </button>
      </div>

      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <div class="flex items-center gap-2 mb-1.5">
          <font-awesome-icon icon="file-import" class="w-3.5 text-accent-primary" />
          <h3 class="m-0 text-sm font-semibold">Import Data</h3>
        </div>
        <p class="text-text-muted text-xs m-0 mb-4">Import memories from file</p>
        <div class="flex flex-col gap-2">
          <input
            v-model="importPath"
            placeholder="File path"
            class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
          />
          <div class="flex gap-2">
            <select
              v-model="importFormat"
              class="flex-1 bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
            >
              <option value="jsonl">JSONL</option>
              <option value="csv">CSV</option>
              <option value="archive">SkyArchive</option>
            </select>
            <button
              class="flex items-center gap-2 px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="importing"
              @click="doImport"
            >
              <font-awesome-icon v-if="importing" icon="circle-notch" class="w-3.5 animate-spin" />
              <font-awesome-icon v-else icon="file-import" class="w-3.5" />
              {{ importing ? 'Importing...' : 'Import' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
      <div class="flex items-center gap-2 mb-4">
        <font-awesome-icon icon="history" class="w-3.5 text-text-muted" />
        <h3 class="m-0 text-sm font-semibold">Snapshots</h3>
        <span class="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] font-mono text-text-muted">{{ backups.length }}</span>
      </div>
      <div v-if="backups.length === 0" class="text-text-muted text-xs text-center py-8">
        No backups yet
      </div>
      <div v-for="snap in backups" :key="snap" class="flex justify-between items-center py-3 border-b border-border-color last:border-0">
        <div class="flex flex-col gap-0.5">
          <span class="font-mono text-xs font-medium text-accent-primary">{{ snap }}</span>
          <span class="text-[10px] text-text-muted">{{ formatSnapshotId(snap) }}</span>
        </div>
        <button
          class="flex items-center gap-1.5 px-3 py-1.5 border border-border-color rounded-lg text-[11px] font-medium cursor-pointer transition-all duration-150 bg-bg-tertiary text-text-secondary hover:border-border-hover hover:text-text-primary disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="restoring.has(snap)"
          @click="restoreBackup(snap)"
        >
          <font-awesome-icon v-if="restoring.has(snap)" icon="circle-notch" class="w-3 animate-spin" />
          <font-awesome-icon v-else icon="rotate" class="w-3" />
          {{ restoring.has(snap) ? 'Restoring...' : 'Restore' }}
        </button>
      </div>
    </div>
  </div>
</template>
