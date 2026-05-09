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
  <div class="dashboard">
    <div class="metrics-grid">
      <div class="metric-card">
        <div class="metric-label">记忆节点</div>
        <div class="metric-value">{{ formatNumber(store.metrics?.total_nodes ?? 0) }}</div>
        <div class="metric-sub">向量存储中的记忆总数</div>
      </div>
      <div class="metric-card">
        <div class="metric-label">关系边</div>
        <div class="metric-value">{{ formatNumber(store.metrics?.total_edges ?? 0) }}</div>
        <div class="metric-sub">图层中的关系连接数</div>
      </div>
      <div class="metric-card accent">
        <div class="metric-label">活跃记忆</div>
        <div class="metric-value">{{ formatNumber(store.metrics?.hot_memories ?? 0) }}</div>
        <div class="metric-sub">高频访问的热记忆</div>
      </div>
      <div class="metric-card warn">
        <div class="metric-label">冷记忆</div>
        <div class="metric-value">{{ formatNumber(store.metrics?.cold_memories ?? 0) }}</div>
        <div class="metric-sub">低频访问待巩固</div>
      </div>
    </div>

    <div class="info-section">
      <div class="info-card">
        <h3>系统配置</h3>
        <div class="info-row">
          <span class="info-label">向量维度</span>
          <span class="info-val">{{ store.info?.dimension ?? '-' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">段数</span>
          <span class="info-val">{{ store.metrics?.segment_count ?? '-' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">备份数</span>
          <span class="info-val">{{ backups.length }}</span>
        </div>
      </div>

      <div class="info-card" v-if="store.info?.property_schema">
        <h3>属性 Schema</h3>
        <div v-for="(types, key) in store.info.property_schema" :key="key" class="info-row">
          <span class="info-label">{{ key }}</span>
          <span class="info-val">
            <span v-for="t in types" :key="t" class="type-badge">{{ t }}</span>
          </span>
        </div>
        <div v-if="Object.keys(store.info.property_schema).length === 0" class="empty-hint">
          暂无属性定义
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.metric-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
  transition: border-color 0.2s;
}

.metric-card:hover {
  border-color: #58a6ff;
}

.metric-card.accent {
  border-left: 3px solid #3fb950;
}

.metric-card.warn {
  border-left: 3px solid #d29922;
}

.metric-label {
  font-size: 13px;
  color: #8b949e;
  margin-bottom: 8px;
}

.metric-value {
  font-size: 32px;
  font-weight: 700;
  color: #e1e4e8;
}

.metric-sub {
  font-size: 12px;
  color: #6e7681;
  margin-top: 6px;
}

.info-section {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 16px;
}

.info-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 20px;
}

.info-card h3 {
  margin: 0 0 16px 0;
  font-size: 15px;
  color: #e1e4e8;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #21262d;
}

.info-row:last-child {
  border-bottom: none;
}

.info-label {
  color: #8b949e;
  font-size: 13px;
}

.info-val {
  color: #e1e4e8;
  font-size: 13px;
  display: flex;
  gap: 4px;
}

.type-badge {
  padding: 2px 6px;
  background: #1c2333;
  border-radius: 4px;
  font-size: 11px;
  color: #58a6ff;
}

.empty-hint {
  color: #6e7681;
  font-size: 13px;
  text-align: center;
  padding: 16px 0;
}
</style>
