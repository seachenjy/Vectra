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
  const theme = ref<'light' | 'dark' | 'system'>('system')

  function getSystemTheme(): 'light' | 'dark' {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  function getEffectiveTheme(): 'light' | 'dark' {
    return theme.value === 'system' ? getSystemTheme() : theme.value
  }

  function setTheme(newTheme: 'light' | 'dark' | 'system') {
    theme.value = newTheme
    localStorage.setItem('admin-theme', newTheme)
    applyTheme()
  }

  function applyTheme() {
    const effective = getEffectiveTheme()
    document.documentElement.setAttribute('data-theme', effective)
  }

  function initTheme() {
    const saved = localStorage.getItem('admin-theme') as 'light' | 'dark' | 'system' | null
    if (saved) {
      theme.value = saved
    }
    applyTheme()

    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (theme.value === 'system') {
        applyTheme()
      }
    })
  }

  function toggleTheme() {
    const current = getEffectiveTheme()
    setTheme(current === 'dark' ? 'light' : 'dark')
  }

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

  return { info, metrics, loading, error, sidebarCollapsed, theme, fetchInfo, fetchMetrics, toggleSidebar, setTheme, initTheme, toggleTheme, getEffectiveTheme }
})
