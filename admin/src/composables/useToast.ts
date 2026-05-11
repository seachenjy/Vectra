import { reactive } from 'vue'

export interface Toast {
  id: number
  type: 'success' | 'error' | 'info'
  message: string
}

let nextId = 0

const toasts = reactive<Toast[]>([])

export function useToast() {
  function add(type: Toast['type'], message: string, duration = 4000) {
    const id = nextId++
    toasts.push({ id, type, message })
    setTimeout(() => remove(id), duration)
  }

  function remove(id: number) {
    const idx = toasts.findIndex(t => t.id === id)
    if (idx !== -1) toasts.splice(idx, 1)
  }

  return {
    toasts,
    success: (message: string) => add('success', message),
    error: (message: string) => add('error', message, 6000),
    info: (message: string) => add('info', message),
    remove,
  }
}
