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
  {"index":0,"distance":0.244...,"metadata":{"source":"s1","created_at":"..."}}
]
```

## Metrics | 距离/相似度函数
| f | type |
|---|---|
| eu | Euclidean Distance |
| l1 | Manhattan Distance |
| cs | Cosine (reported as 1 - cosine) |

Planned | 规划中：`cd` Chebyshev, `md` Minkowski, `js` Jaccard, `mh` Mahalanobis, `hd` Hamming