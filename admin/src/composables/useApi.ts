import axios from 'axios'

const client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE ?? '',
  timeout: 30000,
})

export interface NamespaceInfo {
  name: string
  dimension: number
  node_count: number
  edge_count: number
  created_at: number
}

export interface MemoryItem {
  id: number
  score: number
  distance: number
  vector: number[]
  metadata: Record<string, string>
  memory_type: string
  created_at: number
  access_count: number
  decay_score: number
}

export interface InfoResp {
  node_count: number
  edge_count: number
  dimension: number
  property_schema: Record<string, string[]>
}

export interface SystemMetrics {
  total_nodes: number
  total_edges: number
  segment_count: number
  hot_memories: number
  cold_memories: number
}

export interface EdgeItem {
  from: number
  to: number
  edge_type: string
  weight: number
}

export interface DbConnectionInfo {
  name: string
  db_type: string
  dsn_display: string
  connected: boolean
}

export interface DbQueryResult {
  columns: string[]
  column_types: string[]
  rows: any[][]
  row_count: number
  elapsed_ms: number
}

export interface DbImportResult {
  ok: boolean
  imported: number
  total_rows: number
  errors?: string[]
  error?: string
}

export function useApi() {
  // ── Namespace management ──────────────────────────────────

  async function listNamespaces(): Promise<NamespaceInfo[]> {
    const { data } = await client.get('/api/namespaces')
    return data
  }

  async function createNamespace(name: string, dimension?: number): Promise<NamespaceInfo> {
    const { data } = await client.post('/api/namespaces', { name, dimension })
    return data
  }

  async function deleteNamespace(name: string): Promise<void> {
    await client.delete(`/api/namespaces/${name}`)
  }

  async function getNamespaceInfo(name: string): Promise<NamespaceInfo> {
    const { data } = await client.get(`/api/namespaces/${name}/info`)
    return data
  }

  // ── Info & metrics ────────────────────────────────────────

  async function getInfo(ns: string): Promise<InfoResp> {
    const { data } = await client.get(`/api/ns/${ns}/info`)
    return data
  }

  async function getMetrics(ns: string): Promise<SystemMetrics> {
    const { data } = await client.get(`/api/ns/${ns}/metrics`)
    return data
  }

  // ── Memories ──────────────────────────────────────────────

  async function insertMemory(ns: string, payload: {
    vector: number[]
    metadata: Record<string, string>
    memory_type?: string
  }): Promise<{ ok: boolean; id: number }> {
    const { data } = await client.post(`/api/ns/${ns}/memories`, payload)
    return data
  }

  async function getMemory(ns: string, id: number): Promise<MemoryItem> {
    const { data } = await client.get(`/api/ns/${ns}/memories/${id}`)
    return data
  }

  async function deleteMemory(ns: string, id: number): Promise<void> {
    await client.delete(`/api/ns/${ns}/memories/${id}`)
  }

  async function accessMemory(ns: string, id: number): Promise<void> {
    await client.post(`/api/ns/${ns}/memories/${id}/access`)
  }

  // ── Search & query ────────────────────────────────────────

  async function search(ns: string, payload: {
    vector: number[]
    k?: number
    metric?: string
  }): Promise<MemoryItem[]> {
    const { data } = await client.post(`/api/ns/${ns}/search`, payload)
    return data
  }

  async function queryExec(ns: string, payload: {
    query: string
    k?: number
    metric?: string
  }): Promise<any[]> {
    const { data } = await client.post(`/api/ns/${ns}/query`, payload)
    return data
  }

  // ── Graph ─────────────────────────────────────────────────

  async function addEdge(ns: string, payload: {
    from: number
    to: number
    edge_type: string
    weight?: number
  }): Promise<void> {
    await client.post(`/api/ns/${ns}/edges`, payload)
  }

  async function getEdges(ns: string, id: number): Promise<EdgeItem[]> {
    const { data } = await client.get(`/api/ns/${ns}/edges/${id}`)
    return data
  }

  async function graphTraverse(ns: string, payload: {
    start: number
    depth?: number
    edge_type?: string
  }): Promise<any[]> {
    const { data } = await client.post(`/api/ns/${ns}/graph/traverse`, payload)
    return data
  }

  async function spreadingActivation(ns: string, payload: {
    start: number
    decay_factor?: number
    threshold?: number
    max_hops?: number
  }): Promise<any[]> {
    const { data } = await client.post(`/api/ns/${ns}/graph/activate`, payload)
    return data
  }

  // ── Backup & import/export ────────────────────────────────

  async function handleBackup(ns: string, action: string, snapshot_id?: string): Promise<any> {
    const { data } = await client.post(`/api/ns/${ns}/backup`, { action, snapshot_id })
    return data
  }

  async function listBackups(ns: string): Promise<string[]> {
    const { data } = await client.get(`/api/ns/${ns}/backups`)
    return data
  }

  async function handleImport(ns: string, payload: {
    format: string
    path: string
  }): Promise<any> {
    const { data } = await client.post(`/api/ns/${ns}/import`, payload)
    return data
  }

  async function handleExport(ns: string): Promise<Blob> {
    const { data } = await client.post(`/api/ns/${ns}/export`, {}, { responseType: 'blob' })
    return data
  }

  // ── Database API ──────────────────────────────────────────

  async function dbConnect(payload: {
    name: string
    db_type: string
    dsn: string
    max_connections?: number
  }): Promise<{ ok?: boolean; error?: string }> {
    const { data } = await client.post('/api/db/connect', payload)
    return data
  }

  async function dbDisconnect(name: string): Promise<{ ok?: boolean; error?: string }> {
    const { data } = await client.post('/api/db/disconnect', { name })
    return data
  }

  async function dbListConnections(): Promise<DbConnectionInfo[]> {
    const { data } = await client.get('/api/db/connections')
    return data
  }

  async function dbQuery(connection: string, sql: string): Promise<{ ok?: boolean; data?: DbQueryResult; error?: string }> {
    const { data } = await client.post('/api/db/query', { connection, sql })
    return data
  }

  async function dbListTables(name: string): Promise<{ ok?: boolean; tables?: string[]; error?: string }> {
    const { data } = await client.post('/api/db/tables', { name })
    return data
  }

  async function dbDescribeTable(connection: string, table: string): Promise<{ ok?: boolean; data?: DbQueryResult; error?: string }> {
    const { data } = await client.post('/api/db/describe', { connection, table })
    return data
  }

  async function dbTestConnection(name: string): Promise<{ ok?: boolean; alive?: boolean; error?: string }> {
    const { data } = await client.post('/api/db/test', { name })
    return data
  }

  async function dbImportToVector(ns: string, payload: {
    connection: string
    sql: string
    vector_column: string
    metadata_columns?: string[]
    memory_type?: string
  }): Promise<DbImportResult> {
    const { data } = await client.post(`/api/db/import/${ns}`, payload)
    return data
  }

  // ── Embedding API ─────────────────────────────────────────

  interface EmbeddingStatus {
    text_model: string | null
    image_model: string | null
    text_ready: boolean
    image_ready: boolean
    text_dimension: number
    image_dimension: number
  }

  async function importText(ns: string, payload: {
    text: string
    chunk?: boolean
    chunk_size?: number
    chunk_overlap?: number
    memory_type?: string
    metadata?: Record<string, string>
  }): Promise<{ ok: boolean; imported: number; chunks: number; errors?: string[]; error?: string }> {
    const { data } = await client.post(`/api/ns/${ns}/import/text`, payload, { timeout: 120000 })
    return data
  }

  async function importImage(ns: string, file: File): Promise<{ ok: boolean; id?: number; error?: string }> {
    const { data } = await client.post(`/api/ns/${ns}/import/image`, file, {
      headers: { 'Content-Type': file.type || 'application/octet-stream' },
      timeout: 120000,
    })
    return data
  }

  async function embeddingStatus(): Promise<EmbeddingStatus> {
    const { data } = await client.get('/api/ns/default/embedding/status')
    return data
  }

  return {
    listNamespaces,
    createNamespace,
    deleteNamespace,
    getNamespaceInfo,
    getInfo,
    getMetrics,
    insertMemory,
    getMemory,
    deleteMemory,
    accessMemory,
    search,
    queryExec,
    addEdge,
    getEdges,
    graphTraverse,
    spreadingActivation,
    handleBackup,
    listBackups,
    handleImport,
    handleExport,
    dbConnect,
    dbDisconnect,
    dbListConnections,
    dbQuery,
    dbListTables,
    dbDescribeTable,
    dbTestConnection,
    dbImportToVector,
    importText,
    importImage,
    embeddingStatus,
  }
}
