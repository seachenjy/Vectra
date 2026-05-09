import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { InfoResp, SystemMetrics } from '../composables/useApi'
import { useApi } from '../composables/useApi'

export const useAppStore = defineStore('app', () => {
  const api = useApi()
  const info = ref<InfoResp | null>(null)
  const metrics = ref<SystemMetrics | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const sidebarCollapsed = ref(false)

  async function fetchInfo() {
    loading.value = true
    error.value = null
    try {
      info.value = await api.getInfo()
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function fetchMetrics() {
    try {
      metrics.value = await api.getMetrics()
    } catch (e: any) {
      error.value = e.message
    }
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  return { info, metrics, loading, error, sidebarCollapsed, fetchInfo, fetchMetrics, toggleSidebar }
})
