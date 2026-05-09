<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores/app'
import { useApi } from '../composables/useApi'
import { ref } from 'vue'

const store = useAppStore()
const api = useApi()
const backups = ref<string[]>([])

onMounted(async () => {
  await Promise.all([store.fetchInfo(), store.fetchMetrics()])
  try {
    backups.value = await api.listBackups()
  } catch {}
})

function formatNumber(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M'
  if (n >= 1000) return (n / 1000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <div class="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 transition-all duration-200 hover:border-accent-primary">
        <div class="text-sm text-text-secondary mb-2">记忆节点</div>
        <div class="text-3xl font-bold text-text-primary">{{ formatNumber(store.metrics?.total_nodes ?? 0) }}</div>
        <div class="text-xs text-text-secondary/70 mt-1.5">向量存储中的记忆总数</div>
      </div>
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 transition-all duration-200 hover:border-accent-primary">
        <div class="text-sm text-text-secondary mb-2">关系边</div>
        <div class="text-3xl font-bold text-text-primary">{{ formatNumber(store.metrics?.total_edges ?? 0) }}</div>
        <div class="text-xs text-text-secondary/70 mt-1.5">图层中的关系连接数</div>
      </div>
      <div class="bg-bg-secondary border border-l-2 border-l-green-500 border-border-color rounded-xl p-5 transition-all duration-200 hover:border-accent-primary">
        <div class="text-sm text-text-secondary mb-2">活跃记忆</div>
        <div class="text-3xl font-bold text-text-primary">{{ formatNumber(store.metrics?.hot_memories ?? 0) }}</div>
        <div class="text-xs text-text-secondary/70 mt-1.5">高频访问的热记忆</div>
      </div>
      <div class="bg-bg-secondary border border-l-2 border-l-yellow-500 border-border-color rounded-xl p-5 transition-all duration-200 hover:border-accent-primary">
        <div class="text-sm text-text-secondary mb-2">冷记忆</div>
        <div class="text-3xl font-bold text-text-primary">{{ formatNumber(store.metrics?.cold_memories ?? 0) }}</div>
        <div class="text-xs text-text-secondary/70 mt-1.5">低频访问待巩固</div>
      </div>
    </div>

    <div class="grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-4">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-4 text-base text-text-primary">系统配置</h3>
        <div class="flex justify-between items-center py-2 border-b border-border-color">
          <span class="text-text-secondary text-sm">向量维度</span>
          <span class="text-text-primary text-sm">{{ store.info?.dimension ?? '-' }}</span>
        </div>
        <div class="flex justify-between items-center py-2 border-b border-border-color">
          <span class="text-text-secondary text-sm">段数</span>
          <span class="text-text-primary text-sm">{{ store.metrics?.segment_count ?? '-' }}</span>
        </div>
        <div class="flex justify-between items-center py-2 border-b border-border-color">
          <span class="text-text-secondary text-sm">备份数</span>
          <span class="text-text-primary text-sm">{{ backups.length }}</span>
        </div>
      </div>

      <div v-if="store.info?.property_schema" class="bg-bg-secondary border border-border-color rounded-xl p-5">
        <h3 class="m-0 mb-4 text-base text-text-primary">属性 Schema</h3>
        <div v-for="(types, key) in store.info.property_schema" :key="key" class="flex justify-between items-center py-2 border-b border-border-color">
          <span class="text-text-secondary text-sm">{{ key }}</span>
          <span class="text-text-primary text-sm flex gap-1">
            <span v-for="t in types" :key="t" class="px-1.5 py-0.5 bg-bg-tertiary rounded text-xs text-accent-primary">{{ t }}</span>
          </span>
        </div>
        <div v-if="Object.keys(store.info.property_schema).length === 0" class="text-text-secondary text-sm text-center py-4">
          暂无属性定义
        </div>
      </div>
    </div>
  </div>
</template>
