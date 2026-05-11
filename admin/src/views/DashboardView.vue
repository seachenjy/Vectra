<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useApi } from '../composables/useApi'

const store = useAppStore()
const api = useApi()
const backups = ref<string[]>([])

onMounted(async () => {
  await Promise.all([store.fetchInfo(), store.fetchMetrics()])
  try {
    backups.value = await api.listBackups(store.currentNs)
  } catch {}
})

watch(() => store.currentNs, async () => {
  await Promise.all([store.fetchInfo(), store.fetchMetrics()])
  try {
    backups.value = await api.listBackups(store.currentNs)
  } catch {}
})

function formatNumber(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M'
  if (n >= 1000) return (n / 1000).toFixed(1) + 'K'
  return String(n)
}

const cards = [
  { key: 'total_nodes', label: 'Total Nodes', icon: 'layer-group', color: 'accent-primary' },
  { key: 'total_edges', label: 'Total Edges', icon: 'link', color: 'accent-secondary' },
  { key: 'hot_memories', label: 'Hot Memories', icon: 'fire-flame-curved', color: 'orange-500' },
  { key: 'cold_memories', label: 'Cold Memories', icon: 'snowflake', color: 'blue-400' },
]
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4">
      <div
        v-for="card in cards"
        :key="card.key"
        class="group bg-bg-secondary border border-border-color rounded-xl p-5 transition-all duration-200 hover:border-border-hover hover:shadow-[0_0_20px_rgba(246,130,31,0.05)]"
      >
        <div class="flex items-center justify-between mb-3">
          <span class="text-[11px] font-medium uppercase tracking-wider text-text-muted">{{ card.label }}</span>
          <div class="w-8 h-8 rounded-lg bg-bg-tertiary flex items-center justify-center">
            <font-awesome-icon :icon="card.icon" class="w-3.5 text-text-muted group-hover:text-accent-primary transition-colors" />
          </div>
        </div>
        <div class="text-2xl font-semibold text-text-primary tracking-tight">
          {{ formatNumber((store.metrics as any)?.[card.key] ?? 0) }}
        </div>
      </div>
    </div>

    <div class="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <div class="flex items-center gap-2 mb-4">
          <font-awesome-icon icon="gear" class="w-3.5 text-text-muted" />
          <h3 class="m-0 text-sm font-semibold text-text-primary">Configuration</h3>
        </div>
        <div class="flex flex-col gap-0">
          <div class="flex justify-between items-center py-2.5 border-b border-border-color last:border-0">
            <span class="text-xs text-text-muted">Vector Dimension</span>
            <span class="text-xs text-text-primary font-mono font-medium">{{ store.info?.dimension ?? '-' }}</span>
          </div>
          <div class="flex justify-between items-center py-2.5 border-b border-border-color last:border-0">
            <span class="text-xs text-text-muted">Segments</span>
            <span class="text-xs text-text-primary font-mono font-medium">{{ store.metrics?.segment_count ?? '-' }}</span>
          </div>
          <div class="flex justify-between items-center py-2.5 border-b border-border-color last:border-0">
            <span class="text-xs text-text-muted">Backups</span>
            <span class="text-xs text-text-primary font-mono font-medium">{{ backups.length }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.info?.property_schema" class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <div class="flex items-center gap-2 mb-4">
          <font-awesome-icon icon="sitemap" class="w-3.5 text-text-muted" />
          <h3 class="m-0 text-sm font-semibold text-text-primary">Property Schema</h3>
        </div>
        <div v-for="(types, key) in store.info.property_schema" :key="key" class="flex justify-between items-center py-2.5 border-b border-border-color last:border-0">
          <span class="text-xs text-text-muted font-mono">{{ key }}</span>
          <div class="flex gap-1">
            <span
              v-for="t in types"
              :key="t"
              class="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] font-medium text-accent-primary uppercase tracking-wide"
            >{{ t }}</span>
          </div>
        </div>
        <div v-if="Object.keys(store.info.property_schema).length === 0" class="text-xs text-text-muted text-center py-6">
          No properties defined
        </div>
      </div>
    </div>
  </div>
</template>
