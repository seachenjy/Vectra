# SkyMemory

认知记忆引擎 | Cognitive Memory Engine

SkyMemory 是一个本地优先的认知记忆引擎，融合向量索引、图记忆、时间衰减与语义查询，为 AI Agent 提供类人记忆能力。支持**多命名空间隔离**，每个命名空间拥有独立的向量、图谱和落盘文件。

## 架构概览

```
┌──────────────────────────────────────────────────────────────┐
│   CLI (clap)         REST API (axum)       Admin Panel       │
│                       /api/ns/{ns}/...     (Vue 3 + Tailwind) │
├──────────────────────────────────────────────────────────────┤
│              Namespace Manager (多命名空间隔离)                 │
│         ┌─────────┐  ┌─────────┐  ┌─────────┐               │
│         │  ns-a   │  │  ns-b   │  │ default │               │
│         │ engine  │  │ engine  │  │ engine  │               │
│         └─────────┘  └─────────┘  └─────────┘               │
├──────────────────────────────────────────────────────────────┤
│                      Query Engine                            │
│             (语义解析 + 多路召回 + 评分融合)                     │
├──────────┬──────────┬───────────┬────────────────────────────┤
│  Vector  │  Graph   │ Property  │        Temporal            │
│  (HNSW)  │  (CSR)   │ (Column)  │    (Decay / Consolidate)   │
├──────────┴──────────┴───────────┴────────────────────────────┤
│                    Storage Engine                             │
│               (Segment + WAL + mmap)                          │
└──────────────────────────────────────────────────────────────┘
```

## 核心特性

- **多命名空间** — 运行时动态创建/删除命名空间，每个命名空间独立落盘文件隔离
- **向量索引** — HNSW 近似最近邻索引，x86_64 AVX2 SIMD 加速距离计算
- **图记忆** — CSR 压缩邻接图，BFS 遍历 + 扩散激活
- **时间衰减** — 艾宾浩斯遗忘曲线，冷热分离，记忆巩固
- **语义查询** — 自定义 DSL，属性过滤 + 向量搜索 + 图扩展 + 时间加权
- **数据库导入** — 支持 SQLite / MySQL / PostgreSQL 外部数据源导入
- **管理看板** — Vue 3 + Tailwind CSS 4 暗色主题管理面板

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
| `crates/api` | axum REST API + CORS + 命名空间管理 |
| `crates/database` | 外部数据库连接管理 (SQLite/MySQL/PostgreSQL) |
| `crates/import_export` | JSONL / CSV / SkyArchive 导入导出 |
| `crates/backup` | 快照备份与恢复 |
| `admin/` | Vue 3 管理看板 |

## 快速开始

### 一键启动（推荐）

```bash
# Windows
start.bat

# macOS / Linux
chmod +x start.sh && ./start.sh
```

脚本会自动：
1. 检查并编译后端（release 模式）
2. 安装前端依赖（如未安装）
3. 同时启动后端 API 服务 (`:8080`) 和前端开发服务器 (`:3000`)

### 手动构建

```bash
# 构建后端
cargo build --release

# 启动后端
cargo run --release -- serve --addr 127.0.0.1:8080

# 启动前端（另一个终端）
cd admin && npm install && npm run dev
```

打开浏览器访问 **http://localhost:3000** 即可使用管理看板。

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

## 多命名空间

每个命名空间拥有独立的 QueryEngine、向量索引、图谱和落盘文件。命名空间数据存储在 `data/namespaces/{name}/` 目录下，互不干扰。

```
data/
├── namespaces/
│   ├── default/           ← 内置默认命名空间（不可删除）
│   │   ├── segments/
│   │   ├── backups/
│   │   └── meta.json
│   ├── project-a/         ← 用户创建的命名空间
│   │   ├── segments/
│   │   ├── backups/
│   │   └── meta.json
│   └── project-b/
│       ├── segments/
│       ├── backups/
│       └── meta.json
```

首次启动时，旧版数据（`data/segments/`）会自动迁移到 `namespaces/default/`，保证向后兼容。

### 命名空间命名规则

- 长度 1-64 字符
- 以小写字母或数字开头
- 仅允许小写字母、数字、短横线 `-`、下划线 `_`
- `default` 为系统内置命名空间，不可删除

## REST API

### 命名空间管理

```bash
# 列出所有命名空间
GET /api/namespaces

# 创建命名空间
POST /api/namespaces
{"name": "project-a", "dimension": 128}

# 删除命名空间（含数据）
DELETE /api/namespaces/{ns}

# 获取命名空间信息
GET /api/namespaces/{ns}/info
```

### 记忆管理

```bash
# 插入记忆
POST /api/ns/{ns}/memories
{"vector":[0.1,0.2,0.3], "metadata":{"source":"demo"}, "memory_type":"semantic"}

# 获取记忆
GET /api/ns/{ns}/memories/{id}

# 删除记忆
DELETE /api/ns/{ns}/memories/{id}

# 记录访问（增强衰减分数）
POST /api/ns/{ns}/memories/{id}/access
```

### 搜索与查询

```bash
# 向量相似搜索
POST /api/ns/{ns}/search
{"vector":[0.1,0.2,0.3], "k":10, "metric":"cs"}

# 语义查询
POST /api/ns/{ns}/query
{"query":"BOOST recent AND category=test", "k":5, "metric":"cs"}
```

### 图操作

```bash
# 添加边
POST /api/ns/{ns}/edges
{"from":123, "to":456, "edge_type":"similar_to", "weight":0.9}

# 获取节点的边
GET /api/ns/{ns}/edges/{id}

# 图遍历
POST /api/ns/{ns}/graph/traverse
{"start":123, "depth":2, "edge_type":"similar_to"}

# 扩散激活
POST /api/ns/{ns}/graph/activate
{"start":123, "decay_factor":0.5, "threshold":0.1, "max_hops":3}
```

### 系统信息

```bash
# 系统信息（节点数、边数、维度、属性 Schema）
GET /api/ns/{ns}/info

# 系统指标（热/冷记忆统计）
GET /api/ns/{ns}/metrics
```

### 备份与导入导出

```bash
# 创建快照
POST /api/ns/{ns}/backup
{"action":"create"}

# 恢复快照
POST /api/ns/{ns}/backup
{"action":"restore", "snapshot_id":"snap_xxx"}

# 列出备份
GET /api/ns/{ns}/backups

# 导入
POST /api/ns/{ns}/import
{"format":"jsonl", "path":"/path/to/data.jsonl"}

# 导出（返回 SkyArchive JSON）
POST /api/ns/{ns}/export
```

### 数据库导入

```bash
# 连接外部数据库
POST /api/db/connect
{"name":"mydb", "db_type":"sqlite", "dsn":"path/to/db.sqlite"}

# 断开连接
POST /api/db/disconnect
{"name":"mydb"}

# 列出连接
GET /api/db/connections

# 执行 SQL 查询
POST /api/db/query
{"connection":"mydb", "sql":"SELECT * FROM items LIMIT 100"}

# 列出表
POST /api/db/tables
{"name":"mydb"}

# 描述表结构
POST /api/db/describe
{"connection":"mydb", "table":"items"}

# 测试连接
POST /api/db/test
{"name":"mydb"}

# 从查询结果导入到向量库
POST /api/db/import/{ns}
{"connection":"mydb", "sql":"SELECT * FROM items", "vector_column":"embedding", "memory_type":"semantic"}
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

## 管理看板

管理看板基于 Vue 3 + Tailwind CSS 4 构建，采用 Cloudflare 风格暗色主题。

**功能模块：**
- **Overview** — 系统指标概览、配置信息、属性 Schema
- **Memories** — 向量搜索、插入记忆、查看/删除详情
- **Graph** — 图遍历可视化、扩散激活
- **Query** — 语义查询执行、查询历史、语法示例
- **Import** — 外部数据库连接 (SQLite/MySQL/PostgreSQL)、SQL 查询、数据导入
- **Backups** — 快照创建/恢复、数据导入导出

**命名空间切换：** 顶部导航栏提供命名空间下拉切换器，支持运行时创建/删除命名空间。切换命名空间后所有视图数据自动刷新。

## 技术栈

**后端 (Rust)**
- axum + tokio — 异步 HTTP 服务
- HNSW — 近似最近邻索引
- AVX2 SIMD — 向量距离加速
- bincode — 高效二进制序列化
- rayon — 并行计算
- sqlx — 多数据库驱动 (SQLite / MySQL / PostgreSQL)

**前端 (Vue 3)**
- Vue 3 + Composition API + TypeScript
- Pinia 状态管理（含命名空间管理）
- Vue Router 路由懒加载
- Tailwind CSS 4 暗色主题
- FontAwesome 图标库
- Vite 构建

## License

MIT
