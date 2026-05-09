<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import type { DbConnectionInfo, DbQueryResult } from '../composables/useApi'

const api = useApi()

// ── Connection state ────────────────────────────────────────
const connections = ref<DbConnectionInfo[]>([])
const activeConn = ref<string | null>(null)

const connectForm = ref({
  name: '',
  db_type: 'sqlite',
  dsn: '',
  max_connections: 5,
})

const showConnectForm = ref(false)

// ── Query state ─────────────────────────────────────────────
const sqlText = ref('')
const queryResult = ref<DbQueryResult | null>(null)
const queryLoading = ref(false)

// ── Table browser state ─────────────────────────────────────
const tables = ref<string[]>([])
const selectedTable = ref<string | null>(null)
const tableSchema = ref<DbQueryResult | null>(null)

// ── Import state ────────────────────────────────────────────
const showImportPanel = ref(false)
const importForm = ref({
  vector_column: '',
  metadata_columns: '',
  memory_type: 'semantic',
})

// ── General UI state ────────────────────────────────────────
const loading = ref(false)
const error = ref<string | null>(null)
const success = ref<string | null>(null)

const dsnPlaceholder = computed(() => {
  switch (connectForm.value.db_type) {
    case 'sqlite': return 'path/to/database.db'
    case 'mysql': return 'mysql://user:password@host:3306/dbname'
    case 'postgres': return 'postgres://user:password@host:5432/dbname'
    default: return ''
  }
})

const hasActiveConnection = computed(() => activeConn.value !== null)

// ── Lifecycle ───────────────────────────────────────────────
onMounted(async () => {
  await refreshConnections()
})

// ── Connection actions ──────────────────────────────────────
async function refreshConnections() {
  try {
    connections.value = await api.dbListConnections()
  } catch (e: any) {
    error.value = e.message
  }
}

async function doConnect() {
  if (!connectForm.value.name.trim() || !connectForm.value.dsn.trim()) {
    error.value = '请填写连接名称和 DSN'
    return
  }
  loading.value = true
  error.value = null
  success.value = null
  try {
    const result = await api.dbConnect({
      name: connectForm.value.name,
      db_type: connectForm.value.db_type,
      dsn: connectForm.value.dsn,
      max_connections: connectForm.value.max_connections,
    })
    if (result.error) {
      error.value = result.error
    } else {
      success.value = `已连接: ${connectForm.value.name}`
      activeConn.value = connectForm.value.name
      showConnectForm.value = false
      connectForm.value = { name: '', db_type: 'sqlite', dsn: '', max_connections: 5 }
      await refreshConnections()
      await loadTables()
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function doDisconnect(name: string) {
  loading.value = true
  error.value = null
  try {
    const result = await api.dbDisconnect(name)
    if (result.error) {
      error.value = result.error
    } else {
      if (activeConn.value === name) {
        activeConn.value = null
        tables.value = []
        selectedTable.value = null
        tableSchema.value = null
        queryResult.value = null
      }
      success.value = `已断开: ${name}`
      await refreshConnections()
    }
  } catch (e: any) {
    error.value = e.message
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
  error.value = null
  success.value = null
  try {
    const result = await api.dbTestConnection(name)
    if (result.error) {
      error.value = result.error
    } else {
      success.value = result.alive ? `${name}: 连接正常` : `${name}: 连接失败`
    }
  } catch (e: any) {
    error.value = e.message
  }
}

// ── Table browser actions ───────────────────────────────────
async function loadTables() {
  if (!activeConn.value) return
  try {
    const result = await api.dbListTables(activeConn.value)
    if (result.error) {
      error.value = result.error
    } else {
      tables.value = result.tables ?? []
    }
  } catch (e: any) {
    error.value = e.message
  }
}

async function describeTable(table: string) {
  if (!activeConn.value) return
  selectedTable.value = table
  try {
    const result = await api.dbDescribeTable(activeConn.value, table)
    if (result.error) {
      error.value = result.error
    } else {
      tableSchema.value = result.data ?? null
    }
  } catch (e: any) {
    error.value = e.message
  }
}

function previewTable(table: string) {
  sqlText.value = `SELECT * FROM ${table} LIMIT 100`
  doQuery()
}

// ── Query actions ───────────────────────────────────────────
async function doQuery() {
  if (!activeConn.value || !sqlText.value.trim()) {
    error.value = '请选择连接并输入 SQL'
    return
  }
  queryLoading.value = true
  error.value = null
  success.value = null
  try {
    const result = await api.dbQuery(activeConn.value, sqlText.value)
    if (result.error) {
      error.value = result.error
      queryResult.value = null
    } else {
      queryResult.value = result.data ?? null
    }
  } catch (e: any) {
    error.value = e.message
    queryResult.value = null
  } finally {
    queryLoading.value = false
  }
}

// ── Import actions ──────────────────────────────────────────
function openImportPanel() {
  if (!queryResult.value || queryResult.value.row_count === 0) {
    error.value = '请先执行查询获取数据'
    return
  }
  showImportPanel.value = true
  // Auto-suggest first column as vector column
  if (queryResult.value.columns.length > 0 && !importForm.value.vector_column) {
    importForm.value.vector_column = queryResult.value.columns[0]
  }
}

async function doImport() {
  if (!activeConn.value || !sqlText.value.trim() || !importForm.value.vector_column) {
    error.value = '请填写向量列名'
    return
  }
  loading.value = true
  error.value = null
  success.value = null
  try {
    const metaCols = importForm.value.metadata_columns
      .split(',')
      .map(s => s.trim())
      .filter(Boolean)

    const result = await api.dbImportToVector({
      connection: activeConn.value,
      sql: sqlText.value,
      vector_column: importForm.value.vector_column,
      metadata_columns: metaCols.length > 0 ? metaCols : undefined,
      memory_type: importForm.value.memory_type,
    })
    if (result.error) {
      error.value = result.error
    } else {
      let msg = `导入成功: ${result.imported} / ${result.total_rows} 条记录`
      if (result.errors && result.errors.length > 0) {
        msg += ` (${result.errors.length} 个错误)`
      }
      success.value = msg
      showImportPanel.value = false
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function formatCellValue(val: any): string {
  if (val === null || val === undefined) return 'NULL'
  if (typeof val === 'string') return val
  return JSON.stringify(val)
}

function clearMessages() {
  error.value = null
  success.value = null
}
</script>

<template>
  <div class="db-view" @click="clearMessages">
    <!-- Alerts -->
    <div v-if="error" class="alert error" @click.stop>{{ error }}</div>
    <div v-if="success" class="alert success" @click.stop>{{ success }}</div>

    <!-- Top: Connection management -->
    <div class="section-card">
      <div class="section-header">
        <h3>数据库连接</h3>
        <button class="btn btn-primary btn-sm" @click.stop="showConnectForm = !showConnectForm">
          {{ showConnectForm ? '取消' : '+ 新建连接' }}
        </button>
      </div>

      <!-- Connect form -->
      <div v-if="showConnectForm" class="connect-form" @click.stop>
        <div class="form-row">
          <div class="form-group">
            <label>连接名称</label>
            <input v-model="connectForm.name" class="input-field" placeholder="my_db" />
          </div>
          <div class="form-group">
            <label>数据库类型</label>
            <select v-model="connectForm.db_type" class="input-field">
              <option value="sqlite">SQLite</option>
              <option value="mysql">MySQL</option>
              <option value="postgres">PostgreSQL</option>
            </select>
          </div>
          <div class="form-group narrow">
            <label>最大连接数</label>
            <input v-model.number="connectForm.max_connections" type="number" min="1" max="50" class="input-field" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group flex-1">
            <label>DSN / 连接字符串</label>
            <input v-model="connectForm.dsn" class="input-field" :placeholder="dsnPlaceholder" />
          </div>
          <button class="btn btn-primary" :disabled="loading" @click.stop="doConnect">
            {{ loading ? '连接中...' : '连接' }}
          </button>
        </div>
      </div>

      <!-- Connection list -->
      <div v-if="connections.length > 0" class="conn-list">
        <div
          v-for="conn in connections"
          :key="conn.name"
          :class="['conn-item', { active: activeConn === conn.name }]"
          @click.stop="selectConnection(conn.name)"
        >
          <div class="conn-info">
            <span class="conn-name">{{ conn.name }}</span>
            <span :class="['conn-type-badge', conn.db_type]">{{ conn.db_type }}</span>
            <span :class="['conn-status', conn.connected ? 'online' : 'offline']">
              {{ conn.connected ? 'online' : 'offline' }}
            </span>
          </div>
          <div class="conn-dsn">{{ conn.dsn_display }}</div>
          <div class="conn-actions" @click.stop>
            <button class="btn-icon" title="测试连接" @click="testConnection(conn.name)">&#9889;</button>
            <button class="btn-icon danger" title="断开" @click="doDisconnect(conn.name)">&#10005;</button>
          </div>
        </div>
      </div>
      <div v-else-if="!showConnectForm" class="empty-hint">暂无数据库连接，点击「新建连接」开始</div>
    </div>

    <!-- Main workspace: only show when connected -->
    <template v-if="hasActiveConnection">
      <div class="workspace">
        <!-- Left: Table browser -->
        <div class="table-browser">
          <div class="section-header">
            <h3>表 ({{ tables.length }})</h3>
            <button class="btn-icon" title="刷新" @click="loadTables">&#8635;</button>
          </div>
          <div v-if="tables.length === 0" class="empty-hint">无表</div>
          <div
            v-for="t in tables"
            :key="t"
            :class="['table-item', { active: selectedTable === t }]"
            @click="describeTable(t)"
          >
            <span class="table-name">{{ t }}</span>
            <button class="btn-icon small" title="预览数据" @click.stop="previewTable(t)">&#9654;</button>
          </div>

          <!-- Table schema -->
          <div v-if="tableSchema && selectedTable" class="schema-section">
            <h4>{{ selectedTable }} 结构</h4>
            <div class="schema-table-wrap">
              <table class="data-table compact">
                <thead>
                  <tr>
                    <th v-for="col in tableSchema.columns" :key="col">{{ col }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in tableSchema.rows" :key="ri">
                    <td v-for="(cell, ci) in row" :key="ci">{{ formatCellValue(cell) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Right: SQL editor + results -->
        <div class="query-workspace">
          <!-- SQL Editor -->
          <div class="sql-editor">
            <div class="editor-header">
              <h3>SQL 查询</h3>
              <div class="editor-actions">
                <button class="btn btn-primary btn-sm" :disabled="queryLoading" @click="doQuery">
                  {{ queryLoading ? '执行中...' : '执行 (Ctrl+Enter)' }}
                </button>
                <button
                  v-if="queryResult && queryResult.row_count > 0"
                  class="btn btn-accent btn-sm"
                  @click="openImportPanel"
                >
                  导入向量库
                </button>
              </div>
            </div>
            <textarea
              v-model="sqlText"
              class="sql-input"
              rows="5"
              placeholder="输入 SQL 查询语句..."
              @keydown.ctrl.enter="doQuery"
            ></textarea>
          </div>

          <!-- Import panel -->
          <div v-if="showImportPanel" class="import-panel">
            <div class="section-header">
              <h3>导入到向量库</h3>
              <button class="btn-icon" @click="showImportPanel = false">&#10005;</button>
            </div>
            <p class="hint-text">将查询结果中的数据导入到 SkyMemory 向量库。请指定包含向量数据的列。</p>
            <div class="form-row">
              <div class="form-group">
                <label>向量列 <span class="required">*</span></label>
                <select v-model="importForm.vector_column" class="input-field">
                  <option v-for="col in (queryResult?.columns ?? [])" :key="col" :value="col">{{ col }}</option>
                </select>
              </div>
              <div class="form-group flex-1">
                <label>元数据列 <span class="optional">(逗号分隔，留空=全部)</span></label>
                <input v-model="importForm.metadata_columns" class="input-field" placeholder="col1, col2, col3" />
              </div>
              <div class="form-group">
                <label>记忆类型</label>
                <select v-model="importForm.memory_type" class="input-field">
                  <option value="semantic">语义记忆</option>
                  <option value="episodic">情景记忆</option>
                </select>
              </div>
            </div>
            <div class="form-row">
              <button class="btn btn-primary" :disabled="loading" @click="doImport">
                {{ loading ? '导入中...' : `确认导入 (${queryResult?.row_count ?? 0} 条)` }}
              </button>
            </div>
          </div>

          <!-- Query results -->
          <div v-if="queryResult" class="results-section">
            <div class="results-header">
              <span>{{ queryResult.row_count }} 行 / {{ queryResult.columns.length }} 列</span>
              <span class="elapsed">{{ queryResult.elapsed_ms }}ms</span>
            </div>
            <div class="results-table-wrap">
              <table class="data-table">
                <thead>
                  <tr>
                    <th class="row-num">#</th>
                    <th v-for="(col, i) in queryResult.columns" :key="i">
                      <div class="col-header">
                        <span>{{ col }}</span>
                        <span class="col-type">{{ queryResult.column_types[i] }}</span>
                      </div>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in queryResult.rows" :key="ri">
                    <td class="row-num">{{ ri + 1 }}</td>
                    <td v-for="(cell, ci) in row" :key="ci" :title="formatCellValue(cell)">
                      {{ formatCellValue(cell) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          <div v-else-if="!queryLoading" class="empty-hint center">
            选择表或输入 SQL 查询
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.db-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.alert {
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 13px;
}

.alert.error {
  background: #3d1414;
  border: 1px solid #da3633;
  color: #f85149;
}

.alert.success {
  background: #1c3a2a;
  border: 1px solid #238636;
  color: #3fb950;
}

/* Section cards */
.section-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 16px 20px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.section-header h3 {
  margin: 0;
  font-size: 15px;
}

/* Connect form */
.connect-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 10px;
  margin-bottom: 12px;
}

.form-row {
  display: flex;
  gap: 10px;
  align-items: flex-end;
  flex-wrap: wrap;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-group.flex-1 {
  flex: 1;
  min-width: 200px;
}

.form-group.narrow {
  width: 100px;
}

.form-group label {
  font-size: 12px;
  color: #8b949e;
}

.required {
  color: #f85149;
}

.optional {
  color: #6e7681;
  font-size: 11px;
}

.input-field {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 8px 12px;
  color: #e1e4e8;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

.input-field:focus {
  border-color: #58a6ff;
}

/* Connection list */
.conn-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.conn-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: #0d1117;
  border: 1px solid #21262d;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.conn-item:hover {
  border-color: #58a6ff;
}

.conn-item.active {
  border-color: #58a6ff;
  background: #1c2333;
}

.conn-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.conn-name {
  font-weight: 600;
  font-size: 14px;
  color: #e1e4e8;
}

.conn-type-badge {
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
}

.conn-type-badge.sqlite {
  background: #1c3a2a;
  color: #3fb950;
}

.conn-type-badge.mysql {
  background: #2d1f5e;
  color: #bc8cff;
}

.conn-type-badge.postgres {
  background: #1a2742;
  color: #58a6ff;
}

.conn-status {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 8px;
}

.conn-status.online {
  background: #1c3a2a;
  color: #3fb950;
}

.conn-status.offline {
  background: #3d1414;
  color: #f85149;
}

.conn-dsn {
  font-family: monospace;
  font-size: 11px;
  color: #6e7681;
  flex: 1;
  margin: 0 16px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conn-actions {
  display: flex;
  gap: 4px;
}

/* Workspace layout */
.workspace {
  display: flex;
  gap: 16px;
  min-height: 500px;
}

/* Table browser */
.table-browser {
  width: 240px;
  flex-shrink: 0;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 16px;
  overflow: auto;
  display: flex;
  flex-direction: column;
}

.table-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  font-size: 13px;
  color: #8b949e;
}

.table-item:hover {
  background: #1c2333;
  color: #e1e4e8;
}

.table-item.active {
  background: #1f2a3d;
  color: #58a6ff;
}

.table-name {
  font-family: monospace;
  font-size: 13px;
}

.schema-section {
  margin-top: 16px;
  border-top: 1px solid #30363d;
  padding-top: 12px;
}

.schema-section h4 {
  margin: 0 0 8px 0;
  font-size: 13px;
  color: #8b949e;
}

.schema-table-wrap {
  overflow: auto;
  max-height: 200px;
}

/* Query workspace */
.query-workspace {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.sql-editor {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow: hidden;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid #30363d;
}

.editor-header h3 {
  margin: 0;
  font-size: 14px;
}

.editor-actions {
  display: flex;
  gap: 8px;
}

.sql-input {
  width: 100%;
  background: #0d1117;
  border: none;
  padding: 12px 16px;
  color: #e1e4e8;
  font-size: 14px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  resize: vertical;
  outline: none;
  line-height: 1.6;
}

/* Import panel */
.import-panel {
  background: #161b22;
  border: 1px solid #d29922;
  border-radius: 12px;
  padding: 16px 20px;
}

.import-panel .section-header {
  margin-bottom: 8px;
}

.hint-text {
  color: #8b949e;
  font-size: 13px;
  margin: 0 0 12px 0;
}

/* Results */
.results-section {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow: hidden;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 16px;
  border-bottom: 1px solid #30363d;
  font-size: 12px;
  color: #8b949e;
}

.elapsed {
  color: #3fb950;
  font-family: monospace;
}

.results-table-wrap {
  overflow: auto;
  flex: 1;
}

/* Data table */
.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.data-table.compact {
  font-size: 11px;
}

.data-table th {
  position: sticky;
  top: 0;
  background: #161b22;
  padding: 8px 12px;
  text-align: left;
  border-bottom: 2px solid #30363d;
  font-weight: 600;
  font-size: 12px;
  color: #e1e4e8;
  white-space: nowrap;
}

.data-table td {
  padding: 6px 12px;
  border-bottom: 1px solid #21262d;
  color: #c9d1d9;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.data-table tbody tr:hover {
  background: #1c2333;
}

.col-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.col-type {
  font-size: 10px;
  color: #6e7681;
  font-weight: 400;
  font-family: monospace;
}

.row-num {
  color: #484f58;
  font-size: 11px;
  width: 40px;
  text-align: right;
  font-family: monospace;
}

/* Buttons */
.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-sm {
  padding: 5px 12px;
  font-size: 12px;
}

.btn-primary {
  background: #238636;
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: #2ea043;
}

.btn-accent {
  background: #8957e5;
  color: #fff;
}

.btn-accent:hover:not(:disabled) {
  background: #a371f7;
}

.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 4px 6px;
  border-radius: 4px;
  color: #8b949e;
  transition: all 0.15s;
}

.btn-icon:hover {
  background: #21262d;
  color: #e1e4e8;
}

.btn-icon.danger:hover {
  background: #da3633;
  color: #fff;
}

.btn-icon.small {
  font-size: 12px;
  padding: 2px 4px;
}

.empty-hint {
  color: #6e7681;
  font-size: 13px;
  padding: 16px 0;
}

.empty-hint.center {
  text-align: center;
  padding: 60px 0;
}
</style>
