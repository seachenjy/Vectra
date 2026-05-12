<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const store = useAppStore()

const textContent = ref('')
const textChunk = ref(true)
const textChunkSize = ref(512)
const textChunkOverlap = ref(50)
const textSourceTag = ref('')
const textMemoryType = ref('semantic')
const textLoading = ref(false)

const selectedFile = ref<File | null>(null)
const imagePreview = ref('')
const imageMemoryType = ref('semantic')
const imageLoading = ref(false)
const isDragOver = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

interface ModelStatus {
  text_model: string | null
  image_model: string | null
  text_ready: boolean
  image_ready: boolean
  text_dimension: number
  image_dimension: number
}

const modelStatus = ref<ModelStatus | null>(null)
const statusLoading = ref(false)

onMounted(() => {
  fetchStatus()
})

async function fetchStatus() {
  statusLoading.value = true
  try {
    modelStatus.value = await api.embeddingStatus()
  } catch (e: any) {
    modelStatus.value = null
  } finally {
    statusLoading.value = false
  }
}

async function doImportText() {
  if (!textContent.value.trim()) {
    toast.error(t('importView.enterText'))
    return
  }
  textLoading.value = true
  try {
    const metadata: Record<string, string> = {}
    if (textSourceTag.value.trim()) {
      metadata.source = textSourceTag.value.trim()
    }
    const result = await api.importText(store.currentNs, {
      text: textContent.value,
      chunk: textChunk.value,
      chunk_size: textChunk.value ? textChunkSize.value : undefined,
      chunk_overlap: textChunk.value ? textChunkOverlap.value : undefined,
      memory_type: textMemoryType.value,
      metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    })
    if (result.ok) {
      toast.success(t('importView.textImportSuccess', { count: result.imported }))
      if (result.errors && result.errors.length > 0) {
        for (const err of result.errors) {
          toast.error(err)
        }
      }
      textContent.value = ''
      textSourceTag.value = ''
    } else {
      toast.error(t('importView.textImportError', { error: result.error || 'unknown' }))
    }
  } catch (e: any) {
    toast.error(t('importView.textImportError', { error: e.message }))
  } finally {
    textLoading.value = false
  }
}

function handleFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    setFile(input.files[0])
  }
}

function handleDrop(e: DragEvent) {
  isDragOver.value = false
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
    const file = e.dataTransfer.files[0]
    if (file.type.startsWith('image/')) {
      setFile(file)
    } else {
      toast.error(t('importView.selectImage'))
    }
  }
}

function handleDragOver(e: DragEvent) {
  e.preventDefault()
  isDragOver.value = true
}

function handleDragLeave() {
  isDragOver.value = false
}

function setFile(file: File) {
  selectedFile.value = file
  const reader = new FileReader()
  reader.onload = (e) => {
    imagePreview.value = e.target?.result as string
  }
  reader.readAsDataURL(file)
}

function triggerFileInput() {
  fileInput.value?.click()
}

function clearImage() {
  selectedFile.value = null
  imagePreview.value = ''
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

async function doImportImage() {
  if (!selectedFile.value) {
    toast.error(t('importView.selectImage'))
    return
  }
  imageLoading.value = true
  try {
    const result = await api.importImage(store.currentNs, selectedFile.value)
    if (result.ok) {
      toast.success(t('importView.imageImportSuccess', { id: result.id }))
      clearImage()
    } else {
      toast.error(t('importView.imageImportError', { error: result.error || 'unknown' }))
    }
  } catch (e: any) {
    toast.error(t('importView.imageImportError', { error: e.message }))
  } finally {
    imageLoading.value = false
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2">
          <font-awesome-icon icon="circle-info" class="w-3.5 text-text-muted" />
          <h3 class="m-0 text-sm font-semibold">{{ t('importView.modelStatus') }}</h3>
        </div>
        <button
          class="flex items-center gap-1.5 px-2.5 py-1 border border-border-color rounded-lg text-[11px] font-medium cursor-pointer transition-all duration-150 bg-bg-tertiary text-text-secondary hover:border-border-hover hover:text-text-primary"
          :disabled="statusLoading"
          @click="fetchStatus"
        >
          <font-awesome-icon icon="arrows-rotate" class="w-2.5" :class="{ 'animate-spin': statusLoading }" />
          {{ t('importView.refreshStatus') }}
        </button>
      </div>
      <div v-if="modelStatus" class="grid grid-cols-2 gap-3">
        <div class="bg-bg-primary border border-border-color rounded-lg p-3">
          <div class="flex items-center gap-2 mb-1.5">
            <font-awesome-icon icon="file-alt" class="w-3 text-text-muted" />
            <span class="text-xs font-medium text-text-primary">{{ t('importView.textModel') }}</span>
          </div>
          <div class="font-mono text-[11px] text-text-muted mb-1.5 truncate">
            {{ modelStatus.text_model || '—' }}
          </div>
          <div class="flex items-center gap-2">
            <span class="px-1.5 py-0.5 rounded text-[10px] font-medium"
              :class="modelStatus.text_ready ? 'bg-green-900/50 text-green-400' : 'bg-red-900/50 text-red-400'">
              {{ modelStatus.text_ready ? t('importView.ready') : t('importView.notReady') }}
            </span>
            <span v-if="modelStatus.text_dimension > 0" class="text-[10px] text-text-muted">
              {{ t('importView.embeddingDimension') }}: {{ modelStatus.text_dimension }}
            </span>
          </div>
        </div>
        <div class="bg-bg-primary border border-border-color rounded-lg p-3">
          <div class="flex items-center gap-2 mb-1.5">
            <font-awesome-icon icon="image" class="w-3 text-text-muted" />
            <span class="text-xs font-medium text-text-primary">{{ t('importView.imageModel') }}</span>
          </div>
          <div class="font-mono text-[11px] text-text-muted mb-1.5 truncate">
            {{ modelStatus.image_model || '—' }}
          </div>
          <div class="flex items-center gap-2">
            <span class="px-1.5 py-0.5 rounded text-[10px] font-medium"
              :class="modelStatus.image_ready ? 'bg-green-900/50 text-green-400' : 'bg-red-900/50 text-red-400'">
              {{ modelStatus.image_ready ? t('importView.ready') : t('importView.notReady') }}
            </span>
            <span v-if="modelStatus.image_dimension > 0" class="text-[10px] text-text-muted">
              {{ t('importView.embeddingDimension') }}: {{ modelStatus.image_dimension }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-[1fr_1fr] gap-5">
      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <font-awesome-icon icon="file-alt" class="w-3.5 text-accent-primary" />
          <h3 class="m-0 text-sm font-semibold">{{ t('importView.textImport') }}</h3>
        </div>
        <p class="text-text-muted text-xs m-0">{{ t('importView.textImportDesc') }}</p>

        <textarea
          v-model="textContent"
          :placeholder="t('importView.textPlaceholder')"
          class="w-full bg-bg-primary border border-border-color rounded-lg px-4 py-3 text-text-primary text-[13px] font-mono resize-y outline-none leading-6 transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
          rows="6"
        ></textarea>

        <div class="flex flex-wrap gap-3 items-center">
          <label class="flex items-center gap-1.5 cursor-pointer">
            <input type="checkbox" v-model="textChunk" class="accent-accent-primary" />
            <span class="text-xs text-text-secondary">{{ t('importView.chunkText') }}</span>
          </label>
          <div v-if="textChunk" class="flex gap-2 items-center">
            <input
              v-model.number="textChunkSize"
              type="number"
              min="100"
              max="4096"
              class="w-20 bg-bg-primary border border-border-color rounded-lg px-2.5 py-1.5 text-text-primary text-[12px] outline-none transition-all duration-150 focus:border-accent-primary"
              :placeholder="t('importView.chunkSize')"
            />
            <input
              v-model.number="textChunkOverlap"
              type="number"
              min="0"
              max="500"
              class="w-20 bg-bg-primary border border-border-color rounded-lg px-2.5 py-1.5 text-text-primary text-[12px] outline-none transition-all duration-150 focus:border-accent-primary"
              :placeholder="t('importView.chunkOverlap')"
            />
          </div>
        </div>

        <div class="flex gap-3">
          <div class="flex-1">
            <label class="text-[10px] text-text-muted uppercase tracking-wider mb-1 block">{{ t('importView.sourceTag') }}</label>
            <input
              v-model="textSourceTag"
              :placeholder="t('importView.sourcePlaceholder')"
              class="w-full bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary placeholder:text-text-muted"
            />
          </div>
          <div>
            <label class="text-[10px] text-text-muted uppercase tracking-wider mb-1 block">{{ t('importView.textMemoryType') }}</label>
            <select
              v-model="textMemoryType"
              class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
            >
              <option value="semantic">Semantic</option>
              <option value="episodic">Episodic</option>
            </select>
          </div>
        </div>

        <button
          class="flex items-center justify-center gap-2 px-4 py-2.5 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed mt-auto"
          :disabled="textLoading"
          @click="doImportText"
        >
          <font-awesome-icon v-if="textLoading" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="file-alt" class="w-3.5" />
          {{ textLoading ? t('importView.importingText') : t('importView.importTextButton') }}
        </button>
      </div>

      <div class="bg-bg-secondary border border-border-color rounded-xl p-5 flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <font-awesome-icon icon="image" class="w-3.5 text-accent-primary" />
          <h3 class="m-0 text-sm font-semibold">{{ t('importView.imageImport') }}</h3>
        </div>
        <p class="text-text-muted text-xs m-0">{{ t('importView.imageImportDesc') }}</p>

        <div
          class="flex-1 flex flex-col items-center justify-center border-2 border-dashed rounded-xl cursor-pointer transition-all duration-200 min-h-[200px]"
          :class="isDragOver ? 'border-accent-primary bg-accent-primary/5' : imagePreview ? 'border-border-color' : 'border-border-color hover:border-border-hover'"
          @click="triggerFileInput"
          @drop.prevent="handleDrop"
          @dragover.prevent="handleDragOver"
          @dragleave="handleDragLeave"
        >
          <input
            ref="fileInput"
            type="file"
            accept="image/jpeg,image/png,image/webp"
            class="hidden"
            @change="handleFileSelect"
          />

          <div v-if="imagePreview" class="relative w-full h-full flex items-center justify-center p-4">
            <img :src="imagePreview" class="max-h-[200px] max-w-full object-contain rounded-lg" />
            <div class="absolute bottom-2 left-2 right-2 flex items-center justify-between bg-bg-secondary/90 border border-border-color rounded-lg px-3 py-1.5">
              <span class="text-[11px] text-text-secondary truncate">
                {{ selectedFile?.name }} ({{ formatFileSize(selectedFile?.size ?? 0) }})
              </span>
              <button
                class="text-[10px] text-text-muted bg-transparent border-none cursor-pointer hover:text-red-400 transition-colors"
                @click.stop="clearImage"
              >
                <font-awesome-icon icon="xmark" class="w-3" />
              </button>
            </div>
          </div>

          <div v-else class="flex flex-col items-center gap-3 text-center p-4">
            <div class="w-12 h-12 rounded-full bg-bg-tertiary border border-border-color flex items-center justify-center">
              <font-awesome-icon icon="upload" class="w-5 text-text-muted" />
            </div>
            <div>
              <p class="text-sm text-text-secondary m-0 mb-1">{{ isDragOver ? t('importView.dropHere') : t('importView.dragOrClick') }}</p>
              <p class="text-[10px] text-text-muted m-0">{{ t('importView.supportedFormats') }}</p>
            </div>
          </div>
        </div>

        <select
          v-model="imageMemoryType"
          class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-[13px] outline-none transition-all duration-150 focus:border-accent-primary"
        >
          <option value="semantic">Semantic</option>
          <option value="episodic">Episodic</option>
        </select>

        <button
          class="flex items-center justify-center gap-2 px-4 py-2.5 border-none rounded-lg text-[13px] font-medium cursor-pointer transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="imageLoading || !selectedFile"
          @click="doImportImage"
        >
          <font-awesome-icon v-if="imageLoading" icon="circle-notch" class="w-3.5 animate-spin" />
          <font-awesome-icon v-else icon="image" class="w-3.5" />
          {{ imageLoading ? t('importView.importingImage') : t('importView.importImageButton') }}
        </button>
      </div>
    </div>
  </div>
</template>
