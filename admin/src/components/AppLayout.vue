<script setup lang="ts">
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
</script>

<template>
  <div class="layout">
    <aside :class="['sidebar', { collapsed: store.sidebarCollapsed }]">
      <div class="sidebar-header">
        <span v-if="!store.sidebarCollapsed" class="logo">SkyMemory</span>
        <span v-else class="logo-mini">SM</span>
      </div>
      <nav class="nav">
        <button
          v-for="item in navItems"
          :key="item.path"
          :class="['nav-item', { active: isActive(item.path) }]"
          @click="router.push(item.path)"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span v-if="!store.sidebarCollapsed" class="nav-label">{{ item.label }}</span>
        </button>
      </nav>
      <button class="toggle-btn" @click="store.toggleSidebar">
        {{ store.sidebarCollapsed ? '→' : '←' }}
      </button>
    </aside>
    <main class="main">
      <header class="topbar">
        <h1 class="page-title">{{ route.name }}</h1>
        <div class="status-bar">
          <span v-if="store.info" class="status-chip">
            节点: {{ store.info.node_count }}
          </span>
          <span v-if="store.info" class="status-chip">
            边: {{ store.info.edge_count }}
          </span>
          <span v-if="store.info" class="status-chip">
            维度: {{ store.info.dimension }}
          </span>
          <span v-if="store.loading" class="loading-dot"></span>
        </div>
      </header>
      <section class="content">
        <slot />
      </section>
    </main>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  min-height: 100vh;
  background: #0f1117;
  color: #e1e4e8;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.sidebar {
  width: 220px;
  background: #161b22;
  border-right: 1px solid #30363d;
  display: flex;
  flex-direction: column;
  transition: width 0.2s ease;
  flex-shrink: 0;
}

.sidebar.collapsed {
  width: 60px;
}

.sidebar-header {
  padding: 20px 16px;
  border-bottom: 1px solid #30363d;
}

.logo {
  font-size: 18px;
  font-weight: 700;
  background: linear-gradient(135deg, #58a6ff, #bc8cff);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.logo-mini {
  font-size: 16px;
  font-weight: 700;
  color: #58a6ff;
}

.nav {
  flex: 1;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: #8b949e;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.15s ease;
  text-align: left;
}

.nav-item:hover {
  background: #1c2333;
  color: #e1e4e8;
}

.nav-item.active {
  background: #1f2a3d;
  color: #58a6ff;
}

.nav-icon {
  font-size: 18px;
  flex-shrink: 0;
}

.toggle-btn {
  margin: 8px;
  padding: 8px;
  border: 1px solid #30363d;
  background: #161b22;
  color: #8b949e;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}

.toggle-btn:hover {
  background: #1c2333;
  color: #e1e4e8;
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid #30363d;
  background: #161b22;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  text-transform: capitalize;
}

.status-bar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-chip {
  padding: 4px 10px;
  background: #1c2333;
  border: 1px solid #30363d;
  border-radius: 12px;
  font-size: 12px;
  color: #8b949e;
}

.loading-dot {
  width: 8px;
  height: 8px;
  background: #58a6ff;
  border-radius: 50%;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.content {
  flex: 1;
  padding: 24px;
  overflow: auto;
}
</style>
