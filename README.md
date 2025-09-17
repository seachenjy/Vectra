# Vectra
Simple local vector engine | 本地向量引擎

## CLI Usage | 命令行用法

- Build & Run | 构建运行
```
cargo run -- --help
```

- Create DB | 创建数据库
```
cargo run -- create <name> -d <dimension> [--dir data]
# example 示例
cargo run -- create test -d 3
```

- Insert vector | 插入向量
```
cargo run -- insert <name> -v <v1> <v2> ... [-m k=v,k2=v2] [--dir data]
# examples 示例
cargo run -- insert test -v 1 2 3 -m source=s1
cargo run -- insert test -v 1,2,3 -m source=s1,owner=me
```

- Find nearest | 查询近邻
```
cargo run -- find <name> -v <v1> <v2> ... [-k 10] [-f eu] [--dir data]
# examples 示例
cargo run -- find test -v 1 2 3 -k 5 -f eu
cargo run -- find test -v 1,2,3 -k 5 -f cs
```

Notes | 说明：
- Default data dir is `data/`, configurable via `--dir` | 默认数据目录为 `data/`，可用 `--dir` 指定
- Metadata `-m` supports multiple or comma-separated | 元数据 `-m` 支持多次或逗号分隔
- Supported metrics | 支持的度量：`eu`(欧氏)、`l1`(曼哈顿)、`cs`(余弦，返回 1-cosine)

## REST Server | REST 服务

- Start server | 启动服务
```
cargo run -- serve --addr 127.0.0.1:8080 [--dir data]
```

- Create DB
```
POST /create
{"name":"test","dimension":3}

200 OK
{"ok":true}
```

- Insert vector
```
POST /db/{name}/insert
{"values":[1,2,3],"meta":{"source":"s1"}}

200 OK
{"ok":true,"total":1}
```

- Find nearest
```
POST /db/{name}/find
{"values":[1.1,1.9,3.2],"k":5,"f":"eu"}

200 OK
[
  {"index":0,"distance":0.244...,"values":[1.0,2.0,3.0],"metadata":{"source":"s1","created_at":"..."}}
]
```

Server flags | 服务参数：
```
--addr 127.0.0.1:8080         # listen address | 监听地址
--dir data                    # data directory | 数据目录
--cache-max-mb 128            # memory cap for cache in MB | 缓存最大内存（MB）
--flush-interval-sec 5        # background flush interval | 后台落盘间隔（秒）
--cache-ttl-sec 600           # TTL for idle DBs | 空闲库的生存时间（秒）
```
Notes | 说明：服务内置读通+写回缓存、LRU+TTL 逐出，定期 flush 到磁盘；需要严格一致性可联系维护者启用写穿策略选项。

## Import from SQLite | 从 SQLite 导入

- Command | 命令
```
cargo run -- import-sqlite \
  --sqlite data.db \
  --table my_table \
  --name test \
  --vec-cols v1,v2,v3 \
  --meta-cols source=src_col,owner=user_col \
  --batch-size 200000
```

- Behavior | 行为
- **vec_cols**: columns parsed as vector values `f64`（支持整数/浮点/可解析字符串）
- **meta_cols**: `key=column` mappings; values auto-typed to MetadataValue：
  - Integer → `Integer(i32)`
  - Real → `Float(f32)`
  - Text → `Bool(true/false/1/0)` | `DateTime(RFC3339)` | fallback `String`
- Chunked import into shards `data/<name>_part_*.bin` (configurable by `--batch-size`) | 分片导入保存为多个分片（由 `--batch-size` 控制）

- Example | 示例
```
cargo run -- import-sqlite --sqlite data.db --table items \
  --name products --vec-cols f1,f2,f3 \
  --meta-cols source=src,category=cat,created_at=ts \
  --batch-size 100000

## DB Info | 库信息

- CLI
```
cargo run -- info <name>
# example 示例
cargo run -- info daily
```

- REST
```
GET /db/{name}/info

200 OK
{"name":"daily","dimension":6,"count":6290936,
  "metadata_schema":{"ts_code":["String"],"trade_date":["String"],"created_at":["DateTime"]}}
```
```

## Metrics | 距离/相似度函数
| f | type |
|---|---|
| eu | Euclidean Distance |
| l1 | Manhattan Distance |
| cs | Cosine (reported as 1 - cosine) |

Planned | 规划中：`cd` Chebyshev, `md` Minkowski, `js` Jaccard, `mh` Mahalanobis, `hd` Hamming