import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { InfoResp, SystemMetrics, NamespaceInfo } from '../composables/useApi'
import { useApi } from '../composables/useApi'

export const useAppStore = defineStore('app', () => {
  const api = useApi()
  const info = ref<InfoResp | null>(null)
  const metrics = ref<SystemMetrics | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const sidebarCollapsed = ref(false)
  const theme = ref<'light' | 'dark' | 'system'>('system')

  const namespaces = ref<NamespaceInfo[]>([])
  const currentNs = ref<string>(localStorage.getItem('admin-ns') || 'default')

  const currentNsInfo = computed(() =>
    namespaces.value.find(n => n.name === currentNs.value) ?? null
  )

  function switchNamespace(name: string) {
    currentNs.value = name
    localStorage.setItem('admin-ns', name)
    fetchInfo()
  }

  async function fetchNamespaces() {
    try {
      namespaces.value = await api.listNamespaces()
      if (!namespaces.value.find(n => n.name === currentNs.value)) {
        currentNs.value = namespaces.value[0]?.name ?? 'default'
        localStorage.setItem('admin-ns', currentNs.value)
      }
    } catch {
      namespaces.value = []
    }
  }

  async function createNs(name: string, dimension?: number) {
    const ns = await api.createNamespace(name, dimension)
    namespaces.value.push(ns)
    currentNs.value = ns.name
    localStorage.setItem('admin-ns', ns.name)
    return ns
  }

  async function deleteNs(name: string) {
    await api.deleteNamespace(name)
    namespaces.value = namespaces.value.filter(n => n.name !== name)
    if (currentNs.value === name) {
      currentNs.value = namespaces.value[0]?.name ?? 'default'
      localStorage.setItem('admin-ns', currentNs.value)
    }
  }

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
      info.value = await api.getInfo(currentNs.value)
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function fetchMetrics() {
    try {
      metrics.value = await api.getMetrics(currentNs.value)
    } catch (e: any) {
      error.value = e.message
    }
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  return {
    info, metrics, loading, error, sidebarCollapsed, theme,
    namespaces, currentNs, currentNsInfo,
    fetchInfo, fetchMetrics, fetchNamespaces,
    switchNamespace, createNs, deleteNs,
    toggleSidebar, setTheme, initTheme, toggleTheme, getEffectiveTheme,
  }
})
