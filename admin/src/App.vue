<script setup lang="ts">
import AppLayout from './components/AppLayout.vue'
import { onMounted } from 'vue'
import { useAppStore } from './stores/app'

const store = useAppStore()

onMounted(() => {
  store.initTheme()
  store.fetchInfo()
})
</script>

<template>
  <AppLayout>
    <RouterView />
  </AppLayout>
</template>
