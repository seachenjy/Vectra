<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useRouter, useRoute } from 'vue-router'
import { useToast } from '../composables/useToast'

const store = useAppStore()
const router = useRouter()
const route = useRoute()
const toast = useToast()

const nsDropdownOpen = ref(false)
const showCreateNs = ref(false)
const newNsName = ref('')
const newNsDim = ref<number | undefined>(undefined)

onMounted(async () => {
  await store.fetchNamespaces()
  store.fetchInfo()
})

const navItems = [
  { path: '/dashboard', label: 'Overview', icon: 'chart-line' },
  { path: '/memories', label: 'Memories', icon: 'brain' },
  { path: '/graph', label: 'Graph', icon: 'diagram-project' },
  { path: '/query', label: 'Query', icon: 'magnifying-glass' },
  { path: '/database', label: 'Import', icon: 'database' },
  { path: '/backup', label: 'Backups', icon: 'floppy-disk' },
]

function isActive(path: string) {
  return route.path === path
}

const themeIcon = computed(() => {
  const effective = store.getEffectiveTheme()
  return effective === 'dark' ? 'sun' : 'moon'
})

function selectNs(name: string) {
  store.switchNamespace(name)
  nsDropdownOpen.value = false
}

async function handleCreateNs() {
  if (!newNsName.value.trim()) return
  try {
    await store.createNs(newNsName.value.trim(), newNsDim.value)
    toast.success(`Namespace "${newNsName.value.trim()}" created`)
    newNsName.value = ''
    newNsDim.value = undefined
    showCreateNs.value = false
  } catch (e: any) {
    toast.error(e.response?.data?.error ?? e.message)
  }
}

async function handleDeleteNs(name: string) {
  if (name === 'default') {
    toast.error('Cannot delete the default namespace')
    return
  }
  try {
    await store.deleteNs(name)
    toast.success(`Namespace "${name}" deleted`)
    store.fetchInfo()
  } catch (e: any) {
    toast.error(e.response?.data?.error ?? e.message)
  }
}

function closeDropdown(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.ns-dropdown')) {
    nsDropdownOpen.value = false
  }
}
</script>

<template>
  <div class="flex min-h-screen bg-bg-primary text-text-primary font-sans" @click="closeDropdown">
    <aside
      class="w-[220px] bg-bg-secondary border-r border-border-color flex flex-col transition-all duration-200 flex-shrink-0"
      :class="{ 'w-[56px]': store.sidebarCollapsed }"
    >
      <div class="h-14 px-4 flex items-center border-b border-border-color">
        <span
          v-if="!store.sidebarCollapsed"
          class="text-base font-semibold tracking-tight"
        >
          <span class="text-accent-primary">Sky</span><span>Memory</span>
        </span>
        <span v-else class="text-accent-primary text-base font-bold">SM</span>
      </div>
      <nav class="flex-1 p-3 flex flex-col gap-0.5">
        <button
          v-for="item in navItems"
          :key="item.path"
          class="flex items-center gap-3 px-3 py-2 border-none bg-transparent rounded-lg cursor-pointer text-[13px] font-medium transition-all duration-150"
          :class="isActive(item.path)
            ? 'bg-bg-tertiary text-text-primary border-l-2 border-accent-primary'
            : 'text-text-secondary hover:bg-bg-tertiary hover:text-text-primary'"
          @click="router.push(item.path)"
        >
          <font-awesome-icon :icon="item.icon" class="w-4 text-center flex-shrink-0" />
          <span v-if="!store.sidebarCollapsed">{{ item.label }}</span>
        </button>
      </nav>
      <div class="p-3 border-t border-border-color">
        <button
          class="w-full flex items-center justify-center p-2 border-none bg-transparent text-text-muted rounded-md cursor-pointer text-sm transition-all duration-150 hover:bg-bg-tertiary hover:text-text-secondary"
          @click="store.toggleSidebar"
        >
          <font-awesome-icon :icon="store.sidebarCollapsed ? 'chevron-right' : 'chevron-left'" class="w-3.5" />
        </button>
      </div>
    </aside>
    <main class="flex-1 flex flex-col min-w-0">
      <header class="h-14 flex items-center justify-between px-6 border-b border-border-color bg-bg-secondary">
        <div class="flex items-center gap-4">
          <h1 class="text-sm font-semibold m-0 uppercase tracking-wider text-text-secondary">{{ route.name }}</h1>

          <div class="ns-dropdown relative">
            <button
              class="flex items-center gap-2 px-3 py-1.5 bg-bg-tertiary border border-border-color rounded-md cursor-pointer text-xs font-medium text-text-primary transition-all duration-150 hover:border-border-hover"
              @click.stop="nsDropdownOpen = !nsDropdownOpen"
            >
              <font-awesome-icon icon="folder" class="w-3 text-accent-primary" />
              <span>{{ store.currentNs }}</span>
              <font-awesome-icon icon="chevron-down" class="w-2.5 text-text-muted transition-transform duration-150" :class="{ 'rotate-180': nsDropdownOpen }" />
            </button>

            <div
              v-if="nsDropdownOpen"
              class="absolute top-full left-0 mt-1 w-56 bg-bg-elevated border border-border-color rounded-lg shadow-xl z-50 py-1"
              @click.stop
            >
              <div class="px-3 py-2 text-[10px] uppercase tracking-widest text-text-muted font-semibold">Namespaces</div>
              <button
                v-for="ns in store.namespaces"
                :key="ns.name"
                class="w-full flex items-center justify-between px-3 py-2 bg-transparent border-none cursor-pointer text-[13px] transition-all duration-100"
                :class="ns.name === store.currentNs ? 'bg-bg-tertiary text-accent-primary' : 'text-text-secondary hover:bg-bg-tertiary hover:text-text-primary'"
                @click="selectNs(ns.name)"
              >
                <div class="flex items-center gap-2">
                  <font-awesome-icon icon="folder" class="w-3" />
                  <span class="font-medium">{{ ns.name }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <span class="text-[11px] text-text-muted">{{ ns.node_count }} nodes</span>
                  <button
                    v-if="ns.name !== 'default'"
                    class="w-5 h-5 flex items-center justify-center bg-transparent border-none text-text-muted rounded cursor-pointer hover:text-red-400 hover:bg-red-950/50 transition-all"
                    @click.stop="handleDeleteNs(ns.name)"
                  >
                    <font-awesome-icon icon="trash" class="w-2.5" />
                  </button>
                </div>
              </button>

              <div class="border-t border-border-color mt-1 pt-1">
                <button
                  v-if="!showCreateNs"
                  class="w-full flex items-center gap-2 px-3 py-2 bg-transparent border-none cursor-pointer text-[13px] text-text-muted transition-all duration-100 hover:bg-bg-tertiary hover:text-text-primary"
                  @click="showCreateNs = true"
                >
                  <font-awesome-icon icon="plus" class="w-3" />
                  <span>New namespace</span>
                </button>
                <div v-else class="px-3 py-2 flex flex-col gap-2">
                  <input
                    v-model="newNsName"
                    placeholder="namespace name"
                    class="w-full px-2.5 py-1.5 bg-bg-primary border border-border-color rounded text-xs text-text-primary outline-none focus:border-accent-primary font-mono"
                    @keyup.enter="handleCreateNs"
                  />
                  <input
                    v-model.number="newNsDim"
                    type="number"
                    placeholder="dimension (default: 128)"
                    class="w-full px-2.5 py-1.5 bg-bg-primary border border-border-color rounded text-xs text-text-primary outline-none focus:border-accent-primary font-mono"
                    @keyup.enter="handleCreateNs"
                  />
                  <div class="flex gap-1.5">
                    <button
                      class="flex-1 px-2 py-1.5 bg-accent-primary border-none rounded text-xs font-medium text-white cursor-pointer hover:opacity-90"
                      @click="handleCreateNs"
                    >
                      Create
                    </button>
                    <button
                      class="px-2 py-1.5 bg-bg-tertiary border border-border-color rounded text-xs text-text-muted cursor-pointer hover:text-text-primary"
                      @click="showCreateNs = false; newNsName = ''; newNsDim = undefined"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-3">
          <button
            class="flex items-center justify-center w-8 h-8 border-none bg-transparent text-text-muted rounded-md cursor-pointer transition-all duration-150 hover:bg-bg-tertiary hover:text-text-primary"
            @click="store.toggleTheme"
          >
            <font-awesome-icon :icon="themeIcon" class="w-4" />
          </button>
          <div v-if="store.info" class="flex items-center gap-1.5 px-2.5 py-1 bg-bg-tertiary rounded-md">
            <font-awesome-icon icon="layer-group" class="w-3 text-accent-primary" />
            <span class="text-xs text-text-secondary">{{ store.info.node_count }}</span>
          </div>
          <div v-if="store.info" class="flex items-center gap-1.5 px-2.5 py-1 bg-bg-tertiary rounded-md">
            <font-awesome-icon icon="link" class="w-3 text-accent-primary" />
            <span class="text-xs text-text-secondary">{{ store.info.edge_count }}</span>
          </div>
          <div v-if="store.info" class="flex items-center gap-1.5 px-2.5 py-1 bg-bg-tertiary rounded-md">
            <font-awesome-icon icon="wave-square" class="w-3 text-accent-primary" />
            <span class="text-xs text-text-secondary">{{ store.info.dimension }}d</span>
          </div>
          <span
            v-if="store.loading"
            class="w-2 h-2 bg-accent-primary rounded-full animate-pulse"
          ></span>
        </div>
      </header>
      <section class="flex-1 p-6 overflow-auto">
        <slot />
      </section>
    </main>
    <div class="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      <div
        v-for="t in toast.toasts"
        :key="t.id"
        class="pointer-events-auto flex items-center gap-2.5 px-4 py-3 rounded-lg border text-[13px] font-medium shadow-lg max-w-sm animate-[slideIn_0.25s_ease-out]"
        :class="{
          'bg-green-950/90 border-green-800/50 text-green-300': t.type === 'success',
          'bg-red-950/90 border-red-800/50 text-red-300': t.type === 'error',
          'bg-orange-950/90 border-orange-800/50 text-orange-300': t.type === 'info',
        }"
        @click="toast.remove(t.id)"
      >
        <font-awesome-icon v-if="t.type === 'success'" icon="check" class="w-3.5" />
        <font-awesome-icon v-else-if="t.type === 'error'" icon="xmark" class="w-3.5" />
        <font-awesome-icon v-else icon="info-circle" class="w-3.5" />
        <span>{{ t.message }}</span>
      </div>
    </div>
  </div>
</template>

<style>
@keyframes slideIn {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
</style>
