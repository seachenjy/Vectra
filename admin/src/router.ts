import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/dashboard',
    },
    {
      path: '/dashboard',
      name: 'dashboard',
      component: () => import('./views/DashboardView.vue'),
    },
    {
      path: '/memories',
      name: 'memories',
      component: () => import('./views/MemoriesView.vue'),
    },
    {
      path: '/graph',
      name: 'graph',
      component: () => import('./views/GraphView.vue'),
    },
    {
      path: '/query',
      name: 'query',
      component: () => import('./views/QueryView.vue'),
    },
    {
      path: '/backup',
      name: 'backup',
      component: () => import('./views/BackupView.vue'),
    },
    {
      path: '/import',
      name: 'import',
      component: () => import('./views/ImportView.vue'),
    },
    {
      path: '/database',
      name: 'database',
      component: () => import('./views/DatabaseView.vue'),
    },
  ],
})

export default router
