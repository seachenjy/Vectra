<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useApi } from '../composables/useApi'
import { useToast } from '../composables/useToast'
import { useAppStore } from '../stores/app'
import type { DbConnectionInfo, DbQueryResult } from '../composables/useApi'

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const store = useAppStore()

const connections = ref<DbConnectionInfo[]>([])
const activeConn = ref<string | null>(null)

const connectForm = ref({
  name: '',
  db_type: 'sqlite',
  dsn: '',
  max_connections: 5,
})

const showConnectForm = ref(false)

const sqlText = ref('')
const queryResult = ref<DbQueryResult | null>(null)
const queryLoading = ref(false)

const tables = ref<string[]>([])
const selectedTable = ref<string | null>(null)
const tableSchema = ref<DbQueryResult | null>(null)

const showImportPanel = ref(false)
const importForm = ref({
  vector_column: '',
  metadata_columns: '',
  memory_type: 'semantic',
})

const loading = ref(false)

const dsnPlaceholder = computed(() => {
  switch (connectForm.value.db_type) {
    case 'sqlite': return 'path/to/database.db'
    case 'mysql': return 'mysql://user:password@host:3306/dbname'
    case 'postgres': return 'postgres://user:password@host:5432/dbname'
    default: return ''
  }
})

const hasActiveConnection = computed(() => activeConn.value !== null)

onMounted(async () => {
  await refreshConnections()
})

async function refreshConnections() {
  try {
    connections.value = await api.dbListConnections()
  } catch (e: any) {
    toast.error(e.message)
  }
}

async function doConnect() {
  if (!connectForm.value.name.trim() || !connectForm.value.dsn.trim()) {
    toast.error(t('database.fillNameAndDsn'))
    return
  }
  loading.value = true
  try {
    const result = await api.dbConnect({
      name: connectForm.value.name,
      db_type: connectForm.value.db_type,
      dsn: connectForm.value.dsn,
      max_connections: connectForm.value.max_connections,
    })
    if (result.error) {
      toast.error(result.error)
    } else {
      toast.success(t('database.connected', { name: connectForm.value.name }))
      activeConn.value = connectForm.value.name
      showConnectForm.value = false
      connectForm.value = { name: '', db_type: 'sqlite', dsn: '', max_connections: 5 }
      await refreshConnections()
      await loadTables()
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    loading.value = false
  }
}

async function doDisconnect(name: string) {
  loading.value = true
  try {
    const result = await api.dbDisconnect(name)
    if (result.error) {
      toast.error(result.error)
    } else {
      if (activeConn.value === name) {
        activeConn.value = null
        tables.value = []
        selectedTable.value = null
        tableSchema.value = null
        queryResult.value = null
      }
      toast.success(t('database.disconnected', { name }))
      await refreshConnections()
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    loading.value = false
  }
}

function selectConnection(name: string) {
  activeConn.value = name
  queryResult.value = null
  selectedTable.value = null
  tableSchema.value = null
  loadTables()
}

async function testConnection(name: string) {
  try {
    const result = await api.dbTestConnection(name)
    if (result.error) {
      toast.error(result.error)
    } else {
      toast.info(result.alive ? t('database.connectionOk', { name }) : t('database.connectionFail', { name }))
    }
  } catch (e: any) {
    toast.error(e.message)
  }
}

async function loadTables() {
  if (!activeConn.value) return
  try {
    const result = await api.dbListTables(activeConn.value)
    if (result.error) {
      toast.error(result.error)
    } else {
      tables.value = result.tables ?? []
    }
  } catch (e: any) {
    toast.error(e.message)
  }
}

async function describeTable(table: string) {
  if (!activeConn.value) return
  selectedTable.value = table
  try {
    const result = await api.dbDescribeTable(activeConn.value, table)
    if (result.error) {
      toast.error(result.error)
    } else {
      tableSchema.value = result.data ?? null
    }
  } catch (e: any) {
    toast.error(e.message)
  }
}

function previewTable(table: string) {
  sqlText.value = `SELECT * FROM ${table} LIMIT 100`
  doQuery()
}

async function doQuery() {
  if (!activeConn.value || !sqlText.value.trim()) {
    toast.error(t('database.selectConnectionAndSql'))
    return
  }
  queryLoading.value = true
  try {
    const result = await api.dbQuery(activeConn.value, sqlText.value)
    if (result.error) {
      toast.error(result.error)
      queryResult.value = null
    } else {
      queryResult.value = result.data ?? null
    }
  } catch (e: any) {
    toast.error(e.message)
    queryResult.value = null
  } finally {
    queryLoading.value = false
  }
}

function openImportPanel() {
  if (!queryResult.value || queryResult.value.row_count === 0) {
    toast.error(t('database.queryDataFirst'))
    return
  }
  showImportPanel.value = true
  if (queryResult.value.columns.length > 0 && !importForm.value.vector_column) {
    importForm.value.vector_column = queryResult.value.columns[0]
  }
}

async function doImport() {
  if (!activeConn.value || !sqlText.value.trim() || !importForm.value.vector_column) {
    toast.error(t('database.vectorColumnRequired'))
    return
  }
  loading.value = true
  try {
    const metaCols = importForm.value.metadata_columns
      .split(',')
      .map(s => s.trim())
      .filter(Boolean)

    const result = await api.dbImportToVector(store.currentNs, {
      connection: activeConn.value,
      sql: sqlText.value,
      vector_column: importForm.value.vector_column,
      metadata_columns: metaCols.length > 0 ? metaCols : undefined,
      memory_type: importForm.value.memory_type,
    })
    if (result.error) {
      toast.error(result.error)
    } else {
      let msg = t('database.importSuccess', { imported: result.imported, total: result.total_rows })
      if (result.errors && result.errors.length > 0) {
        msg += t('database.importErrors', { count: result.errors.length })
      }
      toast.success(msg)
      showImportPanel.value = false
    }
  } catch (e: any) {
    toast.error(e.message)
  } finally {
    loading.value = false
  }
}

function formatCellValue(val: any): string {
  if (val === null || val === undefined) return 'NULL'
  if (typeof val === 'string') return val
  return JSON.stringify(val)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="bg-bg-secondary border border-border-color rounded-xl p-5">
      <div class="flex justify-between items-center mb-3">
        <h3 class="m-0 text-sm font-semibold">{{ t('database.dbConnection') }}</h3>
        <button
          class="px-3 py-1.5 border-none rounded-lg text-xs cursor-pointer font-medium transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          @click="showConnectForm = !showConnectForm"
        >
          {{ showConnectForm ? t('database.cancel') : t('database.newConnection') }}
        </button>
      </div>

      <div v-if="showConnectForm" class="flex flex-col gap-2.5 p-4 bg-bg-primary border border-border-color rounded-lg mb-3">
        <div class="flex gap-2.5 flex-wrap items-end">
          <div class="flex flex-col gap-1">
            <label class="text-xs text-text-muted">{{ t('database.connectionName') }}</label>
            <input v-model="connectForm.name" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary" placeholder="my_db" />
          </div>
          <div class="flex flex-col gap-1">
            <label class="text-xs text-text-muted">{{ t('database.dbType') }}</label>
            <select v-model="connectForm.db_type" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary">
              <option value="sqlite">SQLite</option>
              <option value="mysql">MySQL</option>
              <option value="postgres">PostgreSQL</option>
            </select>
          </div>
          <div class="w-[100px] flex flex-col gap-1">
            <label class="text-xs text-text-muted">{{ t('database.maxConnections') }}</label>
            <input v-model.number="connectForm.max_connections" type="number" min="1" max="50" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary" />
          </div>
        </div>
        <div class="flex gap-2.5 flex-wrap items-end">
          <div class="flex-1 min-w-[200px] flex flex-col gap-1">
            <label class="text-xs text-text-muted">{{ t('database.dsn') }}</label>
            <input v-model="connectForm.dsn" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary" :placeholder="dsnPlaceholder" />
          </div>
          <button
            class="px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="loading"
            @click="doConnect"
          >
            {{ loading ? t('database.connecting') : t('database.connect') }}
          </button>
        </div>
      </div>

      <div v-if="connections.length > 0" class="flex flex-col gap-1.5">
        <div
          v-for="conn in connections"
          :key="conn.name"
          class="flex items-center justify-between px-3.5 py-2.5 bg-bg-primary border border-border-color rounded-lg cursor-pointer transition-all duration-150 hover:border-accent-primary"
          :class="{ 'border-accent-primary bg-bg-tertiary': activeConn === conn.name }"
          @click="selectConnection(conn.name)"
        >
          <div class="flex items-center gap-2">
            <span class="font-semibold text-sm text-text-primary">{{ conn.name }}</span>
            <span
              class="px-2 py-0.5 rounded-full text-xs font-medium"
              :class="{
                'bg-green-900/50 text-green-400': conn.db_type === 'sqlite',
                'bg-purple-900/50 text-purple-400': conn.db_type === 'mysql',
                'bg-blue-900/50 text-blue-400': conn.db_type === 'postgres'
              }"
            >
              {{ conn.db_type }}
            </span>
            <span
              class="text-xs px-1.5 py-0.5 rounded"
              :class="conn.connected ? 'bg-green-900/50 text-green-400' : 'bg-red-900/50 text-red-400'"
            >
              {{ conn.connected ? t('database.online') : t('database.offline') }}
            </span>
          </div>
          <div class="font-mono text-xs text-text-muted flex-1 mx-4 overflow-hidden text-ellipsis whitespace-nowrap">{{ conn.dsn_display }}</div>
          <div class="flex gap-1">
            <button class="bg-transparent border-none cursor-pointer text-sm p-1 rounded text-text-muted transition-all duration-150 hover:bg-bg-tertiary hover:text-text-primary" :title="t('database.testConnection')" @click.stop="testConnection(conn.name)">
              <font-awesome-icon icon="bolt" class="w-3" />
            </button>
            <button class="bg-transparent border-none cursor-pointer text-sm p-1 rounded text-text-muted transition-all duration-150 hover:bg-red-600 hover:text-white" :title="t('database.disconnect')" @click.stop="doDisconnect(conn.name)">
              <font-awesome-icon icon="xmark" class="w-3" />
            </button>
          </div>
        </div>
      </div>
      <div v-else-if="!showConnectForm" class="text-text-muted text-sm py-4 text-center">{{ t('database.noConnections') }}</div>
    </div>

    <template v-if="hasActiveConnection">
      <div class="flex gap-4 min-h-[500px]">
        <div class="w-[240px] flex-shrink-0 bg-bg-secondary border border-border-color rounded-xl p-4 overflow-auto flex flex-col">
          <div class="flex justify-between items-center mb-3">
            <h3 class="m-0 text-sm font-semibold">{{ t('database.tables', { count: tables.length }) }}</h3>
            <button class="bg-transparent border-none cursor-pointer p-1 rounded text-text-muted transition-all duration-150 hover:bg-bg-tertiary hover:text-text-primary" :title="t('database.refresh')" @click="loadTables">
              <font-awesome-icon icon="arrows-rotate" class="w-3" />
            </button>
          </div>
          <div v-if="tables.length === 0" class="text-text-muted text-sm">{{ t('database.noTables') }}</div>
          <div
            v-for="tbl in tables"
            :key="tbl"
            class="flex items-center justify-between px-2.5 py-1.5 rounded-md cursor-pointer transition-all duration-150 text-sm text-text-muted hover:bg-bg-tertiary hover:text-text-primary"
            :class="{ 'bg-bg-tertiary text-accent-primary': selectedTable === tbl }"
            @click="describeTable(tbl)"
          >
            <span class="font-mono text-sm">{{ tbl }}</span>
            <button class="bg-transparent border-none cursor-pointer text-xs p-0.5 rounded text-text-muted transition-all duration-150 hover:text-accent-primary" :title="t('database.previewData')" @click.stop="previewTable(tbl)">
              <font-awesome-icon icon="play" class="w-2.5" />
            </button>
          </div>

          <div v-if="tableSchema && selectedTable" class="mt-4 border-t border-border-color pt-3">
            <h4 class="m-0 mb-2 text-sm text-text-muted">{{ t('database.tableStructure', { table: selectedTable }) }}</h4>
            <div class="overflow-auto max-h-[200px]">
              <table class="w-full border-collapse text-xs">
                <thead>
                  <tr>
                    <th v-for="col in tableSchema.columns" :key="col" class="px-2 py-1 text-left font-semibold text-text-primary border-b-2 border-border-color whitespace-nowrap">{{ col }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in tableSchema.rows" :key="ri" class="hover:bg-bg-tertiary">
                    <td v-for="(cell, ci) in row" :key="ci" class="px-2 py-1 border-b border-border-color text-text-primary max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap">{{ formatCellValue(cell) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div class="flex-1 flex flex-col gap-3 min-w-0">
          <div class="bg-bg-secondary border border-border-color rounded-xl overflow-hidden">
            <div class="flex justify-between items-center px-4 py-2.5 border-b border-border-color">
              <h3 class="m-0 text-sm font-semibold">{{ t('database.sqlQuery') }}</h3>
              <div class="flex gap-2">
                <button
                  class="flex items-center gap-1.5 px-3 py-1.5 border-none rounded-lg text-xs cursor-pointer font-medium transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
                  :disabled="queryLoading"
                  @click="doQuery"
                >
                  <font-awesome-icon v-if="queryLoading" icon="circle-notch" class="w-3 animate-spin" />
                  {{ queryLoading ? t('database.running') : t('database.runQuery') }}
                </button>
                <button
                  v-if="queryResult && queryResult.row_count > 0"
                  class="flex items-center gap-1.5 px-3 py-1.5 border-none rounded-lg text-xs cursor-pointer font-medium transition-all duration-150 bg-purple-600 text-white hover:bg-purple-500"
                  @click="openImportPanel"
                >
                  <font-awesome-icon icon="file-import" class="w-3" />
                  {{ t('database.importToVector') }}
                </button>
              </div>
            </div>
            <textarea
              v-model="sqlText"
              class="w-full bg-bg-primary border-none px-4 py-3 text-text-primary text-sm font-mono resize-y outline-none leading-6"
              rows="5"
              :placeholder="t('database.selectTableOrQuery')"
              @keydown.ctrl.enter="doQuery"
            ></textarea>
          </div>

          <div v-if="showImportPanel" class="bg-bg-secondary border border-yellow-600 rounded-xl p-5">
            <div class="flex justify-between items-center mb-2">
              <h3 class="m-0 text-sm font-semibold">{{ t('database.importToVector') }}</h3>
              <button class="bg-transparent border-none cursor-pointer p-1 rounded text-text-muted transition-all duration-150 hover:bg-bg-tertiary" @click="showImportPanel = false">
                <font-awesome-icon icon="xmark" class="w-3.5" />
              </button>
            </div>
            <p class="text-text-muted text-sm m-0 mb-3">{{ t('database.importDataDesc') }}</p>
            <div class="flex gap-2.5 flex-wrap items-end">
              <div class="flex flex-col gap-1">
                <label class="text-xs text-text-muted">{{ t('database.vectorColumn') }} <span class="text-red-400">*</span></label>
                <select v-model="importForm.vector_column" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary">
                  <option v-for="col in (queryResult?.columns ?? [])" :key="col" :value="col">{{ col }}</option>
                </select>
              </div>
              <div class="flex-1 min-w-[200px] flex flex-col gap-1">
                <label class="text-xs text-text-muted">{{ t('database.metadataColumns') }} <span class="text-text-muted/70 text-xs">{{ t('database.metadataColumnsHint') }}</span></label>
                <input v-model="importForm.metadata_columns" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary" placeholder="col1, col2, col3" />
              </div>
              <div class="flex flex-col gap-1">
                <label class="text-xs text-text-muted">{{ t('database.memoryType') }}</label>
                <select v-model="importForm.memory_type" class="bg-bg-primary border border-border-color rounded-lg px-3 py-2 text-text-primary text-sm outline-none transition-all duration-150 focus:border-accent-primary">
                  <option value="semantic">{{ t('database.semanticMemory') }}</option>
                  <option value="episodic">{{ t('database.episodicMemory') }}</option>
                </select>
              </div>
            </div>
            <div class="flex gap-2.5 mt-3">
              <button
                class="flex items-center gap-1.5 px-4 py-2 border-none rounded-lg text-sm cursor-pointer font-medium transition-all duration-150 bg-accent-primary text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
                :disabled="loading"
                @click="doImport"
              >
                <font-awesome-icon v-if="loading" icon="circle-notch" class="w-3.5 animate-spin" />
                {{ loading ? t('database.importing') : t('database.confirmImport', { count: queryResult?.row_count ?? 0 }) }}
              </button>
            </div>
          </div>

          <div v-if="queryResult" class="bg-bg-secondary border border-border-color rounded-xl overflow-hidden flex-1 flex flex-col">
            <div class="flex justify-between items-center px-4 py-2 border-b border-border-color text-xs text-text-muted">
              <span>{{ t('database.rows', { rows: queryResult.row_count, cols: queryResult.columns.length }) }}</span>
              <span class="text-green-400 font-mono">{{ queryResult.elapsed_ms }}ms</span>
            </div>
            <div class="overflow-auto flex-1">
              <table class="w-full border-collapse text-sm">
                <thead>
                  <tr>
                    <th class="w-10 text-right text-text-muted font-mono text-xs px-3 py-2 border-b-2 border-border-color bg-bg-secondary sticky top-0">#</th>
                    <th v-for="(col, i) in queryResult.columns" :key="i" class="px-3 py-2 text-left border-b-2 border-border-color bg-bg-secondary sticky top-0 font-semibold text-text-primary whitespace-nowrap">
                      <div class="flex flex-col gap-0.5">
                        <span>{{ col }}</span>
                        <span class="text-text-muted/70 text-[10px] font-normal font-mono">{{ queryResult.column_types[i] }}</span>
                      </div>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in queryResult.rows" :key="ri" class="hover:bg-bg-tertiary">
                    <td class="text-right text-text-muted font-mono text-xs px-3 py-1.5 border-b border-border-color w-10">{{ ri + 1 }}</td>
                    <td v-for="(cell, ci) in row" :key="ci" class="px-3 py-1.5 border-b border-border-color text-text-primary max-w-[300px] overflow-hidden text-ellipsis whitespace-nowrap" :title="formatCellValue(cell)">
                      {{ formatCellValue(cell) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          <div v-else-if="!queryLoading" class="text-text-muted text-sm text-center py-16">{{ t('database.selectTableOrQuery') }}</div>
        </div>
      </div>
    </template>
  </div>
</template>
