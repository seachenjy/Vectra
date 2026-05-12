<script setup lang="ts">
import { onMounted, ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import { useApi } from '../composables/useApi'

const store = useAppStore()
const api = useApi()
const { t } = useI18n()
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

const metricsData = computed(() => [
  { label: t('dashboard.nodes'), value: store.info?.node_count ?? 0, color: 'bg-blue-500', icon: 'layer-group' },
  { label: t('dashboard.edges'), value: store.info?.edge_count ?? 0, color: 'bg-green-500', icon: 'link' },
  { label: t('dashboard.dimension'), value: store.info?.dimension ?? 0, color: 'bg-purple-500', icon: 'wave-square' },
  { label: t('dashboard.segments'), value: store.metrics?.segment_count ?? 0, color: 'bg-orange-500', icon: 'cubes' },
  { label: t('dashboard.hotMemories'), value: store.metrics?.hot_memories ?? 0, color: 'bg-red-500', icon: 'fire-flame-curved' },
  { label: t('dashboard.coldMemories'), value: store.metrics?.cold_memories ?? 0, color: 'bg-cyan-500', icon: 'snowflake' },
])
</script>

<template>
  <div class="flex flex-col gap-5">
    <h2 class="m-0 text-lg font-semibold">{{ t('dashboard.title') }}</h2>
    <div class="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4">
      <div
        v-for="metric in metricsData"
        :key="metric.label"
        class="bg-bg-secondary border border-border-color rounded-xl p-4 hover:border-border-hover hover:-translate-y-0.5 hover:shadow-lg hover:shadow-accent-primary/5 transition-all duration-200 cursor-default"
      >
        <div class="flex items-center justify-between mb-3">
          <span class="text-xs text-text-muted font-medium uppercase tracking-wider">{{ metric.label }}</span>
          <div class="w-7 h-7 flex items-center justify-center rounded-lg" :class="metric.color">
            <font-awesome-icon :icon="metric.icon" class="w-3.5 text-white" />
          </div>
        </div>
        <div class="text-2xl font-bold text-text-primary">{{ metric.value }}</div>
      </div>
    </div>
    <div class="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-4 text-sm font-semibold flex items-center gap-2">
          <font-awesome-icon icon="gear" class="w-3.5 text-text-muted" />
          {{ t('dashboard.configuration') }}
        </h3>
        <div class="flex flex-col gap-2">
          <div class="flex justify-between items-center py-1.5 border-b border-border-color last:border-0">
            <span class="text-text-secondary text-xs">{{ t('dashboard.vectorDimension') }}</span>
            <span class="font-mono text-xs font-semibold text-accent-primary">{{ store.info?.dimension ?? '-' }}</span>
          </div>
          <div class="flex justify-between items-center py-1.5 border-b border-border-color last:border-0">
            <span class="text-text-secondary text-xs">{{ t('dashboard.currentNamespace') }}</span>
            <span class="font-mono text-xs font-semibold text-accent-primary">{{ store.currentNs }}</span>
          </div>
        </div>
      </div>
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-4 text-sm font-semibold flex items-center gap-2">
          <font-awesome-icon icon="hashtag" class="w-3.5 text-text-muted" />
          {{ t('dashboard.propertySchema') }}
        </h3>
        <div v-if="store.info?.property_schema && Object.keys(store.info.property_schema).length" class="flex flex-col gap-1">
          <div
            v-for="(types, name) in store.info.property_schema"
            :key="name"
            class="flex justify-between items-center py-1.5 border-b border-border-color last:border-0"
          >
            <span class="font-mono text-xs text-text-primary">{{ name }}</span>
            <span class="text-text-muted text-[10px]">{{ types.join(', ') }}</span>
          </div>
        </div>
        <div v-else class="text-text-muted text-xs text-center py-4">{{ t('dashboard.noProperties') }}</div>
      </div>
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-4 text-sm font-semibold flex items-center gap-2">
          <font-awesome-icon icon="history" class="w-3.5 text-text-muted" />
          {{ t('dashboard.latestBackups') }}
        </h3>
        <div v-if="backups.length" class="flex flex-col gap-1">
          <div
            v-for="snap in backups.slice(0, 5)"
            :key="snap"
            class="flex items-center gap-2 py-1.5 border-b border-border-color last:border-0"
          >
            <font-awesome-icon icon="floppy-disk" class="w-3 text-accent-primary" />
            <span class="font-mono text-xs text-text-secondary truncate">{{ snap }}</span>
          </div>
        </div>
        <div v-else class="text-text-muted text-xs text-center py-4">{{ t('dashboard.noBackups') }}</div>
      </div>
    </div>
  </div>
</template>
