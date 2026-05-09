# SkyMemory

认知记忆引擎 | Cognitive Memory Engine

SkyMemory 是一个本地优先的认知记忆引擎，融合向量索引、图记忆、时间衰减与语义查询，为 AI Agent 提供类人记忆能力。

## 架构概览

```
┌─────────────────────────────────────────────────────────┐
│   CLI (clap)          REST API (axum)     Admin Panel   │
├─────────────────────────────────────────────────────────┤
│                   Query Engine                          │
│          (语义解析 + 多路召回 + 评分融合)                  │
├──────────┬──────────┬───────────┬───────────────────────┤
│  Vector  │  Graph   │ Property  │      Temporal         │
│  (HNSW)  │  (CSR)   │ (Column)  │   (Decay/Consolidate) │
├──────────┴──────────┴───────────┴───────────────────────┤
│                 Storage Engine                           │
│            (Segment + WAL + mmap)                        │
└─────────────────────────────────────────────────────────┘
```

## Workspace 结构

| Crate | 说明 |
|-------|------|
| `skymemory` | CLI 入口 + 服务器启动 |
| `crates/core` | 共享类型、错误定义 |
| `crates/storage` | 段式存储引擎 + WAL 预写日志 |
| `crates/vector` | HNSW 向量索引 + AVX2 SIMD 距离计算 |
| `crates/graph` | CSR 压缩邻接图 + BFS 遍历 + 扩散激活 |
| `crates/property` | 列式属性存储 + 范围查询索引 |
| `crates/temporal` | 艾宾浩斯衰减 + 冷热分离 + 记忆巩固 |
| `crates/query` | DSL 解析器 + 多路查询规划器 |
| `crates/api` | axum REST API + CORS |
| `crates/import_export` | JSONL / CSV / SkyArchive 导入导出 |
| `crates/backup` | 快照备份与恢复 |
| `admin/` | Vue 3 管理看板 |

## 快速开始

### 构建

```bash
cargo build --release
```

### CLI 用法

```bash
# 插入记忆
skymemory insert 0.1 0.2 0.3 --meta source=demo,topic=test --memory-type semantic

# 向量搜索
skymemory search 0.1 0.2 0.3 -k 10 -m cs

# 语义查询
skymemory query "BOOST recent" -k 5

# 建立关系
skymemory edge 123456 789012 --edge-type similar_to --weight 0.9

# 查看系统信息
skymemory info

# 导出数据
skymemory export -o backup.json

# 导入数据
skymemory import -p data.jsonl --format jsonl

# 创建快照
skymemory backup

# 恢复快照
skymemory restore snap_1234567890

# 查看所有备份
skymemory list-backups
```

### 启动 REST 服务

```bash
skymemory serve --addr 127.0.0.1:8080
```

### 启动管理看板

```bash
cd admin
npm install
npm run dev
# → http://localhost:3000
```

管理看板通过 Vite 代理将 `/api` 请求转发到后端 `:8080`。

## REST API

### 记忆管理

```bash
# 插入记忆
POST /api/memories
{"vector":[0.1,0.2,0.3], "metadata":{"source":"demo"}, "memory_type":"semantic"}

# 获取记忆
GET /api/memories/{id}

# 删除记忆
DELETE /api/memories/{id}

# 记录访问（增强衰减分数）
POST /api/memories/{id}/access
```

### 搜索与查询

```bash
# 向量相似搜索
POST /api/search
{"vector":[0.1,0.2,0.3], "k":10, "metric":"cs"}

# 语义查询
POST /api/query
{"query":"BOOST recent AND category=test", "k":5, "metric":"cs"}
```

### 图操作

```bash
# 添加边
POST /api/edges
{"from":123, "to":456, "edge_type":"similar_to", "weight":0.9}

# 获取节点的边
GET /api/edges/{id}

# 图遍历
POST /api/graph/traverse
{"start":123, "depth":2, "edge_type":"similar_to"}

# 扩散激活
POST /api/graph/activate
{"start":123, "decay_factor":0.5, "threshold":0.1, "max_hops":3}
```

### 系统信息

```bash
# 系统信息（节点数、边数、维度、属性 Schema）
GET /api/info

# 系统指标（热/冷记忆统计）
GET /api/metrics
```

### 备份与导入导出

```bash
# 创建快照
POST /api/backup
{"action":"create"}

# 恢复快照
POST /api/backup
{"action":"restore", "snapshot_id":"snap_xxx"}

# 列出备份
GET /api/backups

# 导入
POST /api/import
{"format":"jsonl", "path":"/path/to/data.jsonl"}

# 导出（返回 SkyArchive JSON）
POST /api/export
```

## 距离度量

| 代码 | 名称 | 说明 |
|------|------|------|
| `eu` | Euclidean | 欧氏距离 |
| `cs` | Cosine | 余弦距离 (1 - cos) |
| `dot` | Dot Product | 负点积距离 |

向量距离计算支持 x86_64 AVX2 SIMD 加速。

## 语义查询 DSL

查询语法支持以下操作符组合：

```
VECTOR_SIMILAR("text")     向量相似搜索
key = value                 属性精确过滤
key > value                 属性范围过滤
BOOST recent [weight]       时间近因性加权
EXPAND depth=N              图关系扩展
A AND B                     交集
A OR B                      并集
```

示例：

```bash
# 搜索最近的语义记忆
skymemory query "BOOST recent" -k 10

# 属性过滤 + 时间加权
skymemory query "category = technology AND BOOST recent 2.0" -k 5

# 图扩展查询
skymemory query "EXPAND depth=2" -k 10
```

## 记忆类型

| 类型 | 说明 |
|------|------|
| `Semantic` | 语义记忆 — 事实性知识，衰减较慢 |
| `Episodic` | 情景记忆 — 事件性记录，依赖访问频率维持 |

## 边类型

| 类型 | 说明 |
|------|------|
| `SimilarTo` | 相似关系 |
| `DerivedFrom` | 派生关系 |
| `PartOf` | 组成关系 |
| `TemporallyAfter` | 时序关系 |
| `Contradicts` | 矛盾关系 |
| `References` | 引用关系 |

## 存储格式

### SkyArchive (.json)

导出格式为标准 JSON，包含 manifest + 完整节点数据：

```json
{
  "manifest": {
    "version": "0.2.0",
    "node_count": 100,
    "dimension": 128,
    "created_at": 1700000000000
  },
  "nodes": [...]
}
```

### 数据目录结构

```
data/
├── segments/
│   └── seg_xxx/
│       ├── vectors.bin    序列化节点数据
│       ├── meta.json      段元信息
│       └── wal.log        预写日志
└── backups/
    └── snap_xxx.json      快照文件
```

## 技术栈

**后端 (Rust)**
- axum + tokio — 异步 HTTP 服务
- HNSW — 近似最近邻索引
- AVX2 SIMD — 向量距离加速
- bincode — 高效二进制序列化
- rayon — 并行计算

**前端 (Vue 3)**
- Vue 3 + Composition API + TypeScript
- Pinia 状态管理
- Vue Router 路由懒加载
- Axios API 调用
- Vite 构建

## License

MIT
