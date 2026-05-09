import axios from 'axios'

const client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE ?? '',
  timeout: 30000,
})

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

export function useApi() {
  async function getInfo(): Promise<InfoResp> {
    const { data } = await client.get('/api/info')
    return data
  }

  async function getMetrics(): Promise<SystemMetrics> {
    const { data } = await client.get('/api/metrics')
    return data
  }

  async function insertMemory(payload: {
    vector: number[]
    metadata: Record<string, string>
    memory_type?: string
  }): Promise<{ ok: boolean; id: number }> {
    const { data } = await client.post('/api/memories', payload)
    return data
  }

  async function getMemory(id: number): Promise<MemoryItem> {
    const { data } = await client.get(`/api/memories/${id}`)
    return data
  }

  async function deleteMemory(id: number): Promise<void> {
    await client.delete(`/api/memories/${id}`)
  }

  async function accessMemory(id: number): Promise<void> {
    await client.post(`/api/memories/${id}/access`)
  }

  async function search(payload: {
    vector: number[]
    k?: number
    metric?: string
  }): Promise<MemoryItem[]> {
    const { data } = await client.post('/api/search', payload)
    return data
  }

  async function queryExec(payload: {
    query: string
    k?: number
    metric?: string
  }): Promise<any[]> {
    const { data } = await client.post('/api/query', payload)
    return data
  }

  async function addEdge(payload: {
    from: number
    to: number
    edge_type: string
    weight?: number
  }): Promise<void> {
    await client.post('/api/edges', payload)
  }

  async function getEdges(id: number): Promise<EdgeItem[]> {
    const { data } = await client.get(`/api/edges/${id}`)
    return data
  }

  async function graphTraverse(payload: {
    start: number
    depth?: number
    edge_type?: string
  }): Promise<any[]> {
    const { data } = await client.post('/api/graph/traverse', payload)
    return data
  }

  async function spreadingActivation(payload: {
    start: number
    decay_factor?: number
    threshold?: number
    max_hops?: number
  }): Promise<any[]> {
    const { data } = await client.post('/api/graph/activate', payload)
    return data
  }

  async function handleBackup(action: string, snapshot_id?: string): Promise<any> {
    const { data } = await client.post('/api/backup', { action, snapshot_id })
    return data
  }

  async function listBackups(): Promise<string[]> {
    const { data } = await client.get('/api/backups')
    return data
  }

  async function handleImport(payload: {
    format: string
    path: string
  }): Promise<any> {
    const { data } = await client.post('/api/import', payload)
    return data
  }

  async function handleExport(): Promise<Blob> {
    const { data } = await client.post('/api/export', {}, { responseType: 'blob' })
    return data
  }

  return {
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
  }
}
