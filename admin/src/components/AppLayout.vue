<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '../stores/app'
import { useRouter, useRoute } from 'vue-router'

const store = useAppStore()
const router = useRouter()
const route = useRoute()

const navItems = [
  { path: '/dashboard', label: '总览', icon: '📊' },
  { path: '/memories', label: '记忆管理', icon: '🧠' },
  { path: '/graph', label: '图谱可视化', icon: '🔗' },
  { path: '/query', label: '查询控制台', icon: '🔍' },
  { path: '/database', label: '数据库导入', icon: '🗄' },
  { path: '/backup', label: '备份管理', icon: '💾' },
]

function isActive(path: string) {
  return route.path === path
}

const themeIcon = computed(() => {
  const effective = store.getEffectiveTheme()
  return effective === 'dark' ? '☀️' : '🌙'
})

const themeLabel = computed(() => {
  if (store.theme === 'system') return '跟随系统'
  return store.getEffectiveTheme() === 'dark' ? '暗色' : '亮色'
})
</script>

<template>
  <div class="flex min-h-screen bg-bg-primary text-text-primary font-sans">
    <aside
      class="w-[220px] bg-bg-secondary border-r border-border-color flex flex-col transition-all duration-200 flex-shrink-0"
      :class="{ 'w-[60px]': store.sidebarCollapsed }"
    >
      <div class="p-5 border-b border-border-color">
        <span
          v-if="!store.sidebarCollapsed"
          class="text-lg font-bold bg-gradient-to-r from-accent-primary to-accent-secondary bg-clip-text text-transparent"
        >
          SkyMemory
        </span>
        <span v-else class="text-base font-bold text-accent-primary">SM</span>
      </div>
      <nav class="flex-1 p-2 flex flex-col gap-1">
        <button
          v-for="item in navItems"
          :key="item.path"
          class="flex items-center gap-2.5 p-2.5 border-none bg-transparent text-text-secondary rounded-lg cursor-pointer text-sm transition-all duration-150 hover:bg-bg-tertiary hover:text-text-primary"
          :class="{ 'bg-bg-elevated text-accent-primary': isActive(item.path) }"
          @click="router.push(item.path)"
        >
          <span class="text-lg flex-shrink-0">{{ item.icon }}</span>
          <span v-if="!store.sidebarCollapsed">{{ item.label }}</span>
        </button>
      </nav>
      <button
        class="m-2 p-2 border border-bg-secondary bg-bg-secondary text-text-secondary rounded-md cursor-pointer text-sm hover:bg-bg-tertiary hover:text-text-primary"
        @click="store.toggleSidebar"
      >
        {{ store.sidebarCollapsed ? '→' : '←' }}
      </button>
    </aside>
    <main class="flex-1 flex flex-col min-w-0">
      <header class="flex items-center justify-between p-4 border-b border-border-color bg-bg-secondary">
        <h1 class="text-xl font-semibold m-0 capitalize">{{ route.name }}</h1>
        <div class="flex items-center gap-2.5">
          <button
            class="flex items-center justify-center w-8 h-8 border border-border-color bg-bg-tertiary rounded-md cursor-pointer transition-all duration-150 hover:bg-bg-elevated"
            @click="store.toggleTheme"
            :title="themeLabel"
          >
            <span class="text-base">{{ themeIcon }}</span>
          </button>
          <span
            v-if="store.info"
            class="px-2.5 py-1 bg-bg-tertiary border border-border-color rounded-full text-xs text-text-secondary"
          >
            节点: {{ store.info.node_count }}
          </span>
          <span
            v-if="store.info"
            class="px-2.5 py-1 bg-bg-tertiary border border-border-color rounded-full text-xs text-text-secondary"
          >
            边: {{ store.info.edge_count }}
          </span>
          <span
            v-if="store.info"
            class="px-2.5 py-1 bg-bg-tertiary border border-border-color rounded-full text-xs text-text-secondary"
          >
            维度: {{ store.info.dimension }}
          </span>
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
  </div>
</template>
