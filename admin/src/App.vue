<script setup lang="ts">
import AppLayout from './components/AppLayout.vue'
import { onMounted } from 'vue'
import { useAppStore } from './stores/app'

const store = useAppStore()

onMounted(() => {
  store.fetchInfo()
})
</script>

<template>
  <AppLayout>
    <RouterView />
  </AppLayout>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: #0f1117;
  color: #e1e4e8;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: #161b22;
}

::-webkit-scrollbar-thumb {
  background: #30363d;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: #484f58;
}
</style>
