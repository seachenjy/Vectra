use axum::extract::{Path, State};
use axum::http::Method;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use skymemory_core::{Edge, EdgeType, MemoryNode, MemoryType, Metric, Property, Value, generate_node_id, now_millis, TemporalInfo};
use skymemory_query::parse_query;
use skymemory_import_export as import_export;
use skymemory_database::{DatabaseManager, DbConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::namespace::NamespaceManager;
use crate::types::*;

struct AppStateInner {
    namespaces: NamespaceManager,
    db_manager: DatabaseManager,
    default_dimension: usize,
}

type AppState = Arc<AppStateInner>;

pub async fn build_router(namespaces: NamespaceManager, dimension: usize) -> Router {
    namespaces.initialize().await.expect("failed to initialize namespaces");

    let state: AppState = Arc::new(AppStateInner {
        namespaces,
        db_manager: DatabaseManager::new(),
        default_dimension: dimension,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        .route("/api/namespaces", get(list_namespaces))
        .route("/api/namespaces", post(create_namespace))
        .route("/api/namespaces/:ns", delete(delete_namespace))
        .route("/api/namespaces/:ns/info", get(namespace_info))
        .route("/api/ns/:ns/memories", post(insert_memory))
        .route("/api/ns/:ns/memories/:id", get(get_memory))
        .route("/api/ns/:ns/memories/:id", delete(delete_memory))
        .route("/api/ns/:ns/memories/:id/access", post(access_memory))
        .route("/api/ns/:ns/search", post(search))
        .route("/api/ns/:ns/query", post(query_exec))
        .route("/api/ns/:ns/info", get(info))
        .route("/api/ns/:ns/metrics", get(metrics))
        .route("/api/ns/:ns/edges", post(add_edge))
        .route("/api/ns/:ns/edges/:id", get(get_edges))
        .route("/api/ns/:ns/backup", post(handle_backup))
        .route("/api/ns/:ns/backups", get(list_backups))
        .route("/api/ns/:ns/import", post(handle_import))
        .route("/api/ns/:ns/export", post(handle_export))
        .route("/api/ns/:ns/graph/traverse", post(graph_traverse))
        .route("/api/ns/:ns/graph/activate", post(spreading_activation))
        .route("/api/db/connect", post(db_connect))
        .route("/api/db/disconnect", post(db_disconnect))
        .route("/api/db/connections", get(db_list_connections))
        .route("/api/db/query", post(db_query))
        .route("/api/db/tables", post(db_list_tables))
        .route("/api/db/describe", post(db_describe_table))
        .route("/api/db/test", post(db_test_connection))
        .route("/api/db/import/:ns", post(db_import_to_vector))
        .layer(cors)
        .with_state(state)
}

async fn list_namespaces(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let list = state.namespaces.list_namespaces().await;
    Json(json!(list))
}

#[derive(Deserialize)]
struct CreateNamespaceReq {
    name: String,
    dimension: Option<usize>,
}

async fn create_namespace(
    State(state): State<AppState>,
    Json(req): Json<CreateNamespaceReq>,
) -> impl IntoResponse {
    let dim = req.dimension.unwrap_or(state.default_dimension);
    match state.namespaces.create_namespace(&req.name, dim).await {
        Ok(info) => Json(json!(info)),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn delete_namespace(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.namespaces.delete_namespace(&name).await {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn namespace_info(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.namespaces.get_namespace(&name).await {
        Some(ns) => Json(json!(ns.info())),
        None => Json(json!({"error": format!("namespace '{}' not found", name)})),
    }
}

async fn insert_memory(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<CreateMemoryReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let mem_type = match req.memory_type.as_deref() {
        Some("episodic") => MemoryType::Episodic,
        _ => MemoryType::Semantic,
    };
    let properties: Vec<Property> = req.metadata.iter().map(|(k, v)| Property {
        key: k.clone(),
        value: Value::String(v.clone()),
    }).collect();
    let node = MemoryNode {
        id: generate_node_id(),
        vector: req.vector,
        properties,
        temporal: TemporalInfo::default(),
        memory_type: mem_type,
    };
    let id = node.id;
    match ns.engine.insert_node(node) {
        Ok(()) => Json(json!({"ok": true, "id": id})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_memory(
    State(state): State<AppState>,
    Path((ns_name, id)): Path<(String, u64)>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    match ns.engine.get_node(id) {
        Some(node) => {
            let metadata: HashMap<String, String> = node.properties.iter()
                .map(|p| (p.key.clone(), p.value.to_string()))
                .collect();
            Json(json!(MemoryItem {
                id: node.id,
                score: 0.0,
                distance: 0.0,
                vector: node.vector,
                metadata,
                memory_type: format!("{:?}", node.memory_type),
                created_at: node.temporal.created_at,
                access_count: node.temporal.access_count,
                decay_score: node.temporal.decay_score,
            }))
        }
        None => Json(json!({"error": "not found"})),
    }
}

async fn delete_memory(
    State(state): State<AppState>,
    Path((ns_name, id)): Path<(String, u64)>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    ns.engine.remove_node(id);
    Json(json!({"ok": true}))
}

async fn access_memory(
    State(state): State<AppState>,
    Path((ns_name, id)): Path<(String, u64)>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let mut ts = ns.engine.temporal_store.write();
    ts.record_access(id);
    Json(json!({"ok": true}))
}

async fn search(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<SearchReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let engine = &ns.engine;
    let metric = Metric::from_code(req.metric.as_deref().unwrap_or("cs")).unwrap_or(Metric::Cosine);
    let k = req.k.unwrap_or(10);
    let node_data = engine.node_data.read();
    if node_data.is_empty() {
        return Json(json!([]));
    }
    let raw = engine.vector_store.search(&req.vector, k * 2, &metric).unwrap_or_default();
    let mut results: Vec<MemoryItem> = Vec::new();
    for (node_id, dist) in raw {
        if let Some(node) = node_data.get(&node_id) {
            let metadata: HashMap<String, String> = node.properties.iter()
                .map(|p| (p.key.clone(), p.value.to_string()))
                .collect();
            let temporal_score = engine.temporal_store.read().forgetting_curve_score(node_id, now_millis());
            let score = (1.0 / (1.0 + dist as f32)) * temporal_score;
            results.push(MemoryItem {
                id: node_id,
                score,
                distance: dist,
                vector: node.vector.clone(),
                metadata,
                memory_type: format!("{:?}", node.memory_type),
                created_at: node.temporal.created_at,
                access_count: node.temporal.access_count,
                decay_score: node.temporal.decay_score,
            });
        }
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(k);
    Json(json!(results))
}

async fn query_exec(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<QueryReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let parsed = match parse_query(&req.query) {
        Ok(q) => q,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };
    let metric = Metric::from_code(req.metric.as_deref().unwrap_or("cs")).unwrap_or(Metric::Cosine);
    let k = req.k.unwrap_or(10);
    let results = match ns.engine.execute(&parsed, k, &metric) {
        Ok(r) => r,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };
    let items: Vec<serde_json::Value> = results.iter().map(|r| {
        let metadata: HashMap<String, String> = r.properties.iter()
            .map(|p| (p.key.clone(), p.value.to_string()))
            .collect();
        json!({
            "id": r.node_id,
            "score": r.score,
            "distance": r.distance,
            "metadata": metadata,
            "memory_type": format!("{:?}", r.memory_type),
            "expanded_from": r.expanded_from,
        })
    }).collect();
    Json(json!(items))
}

async fn info(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let node_count = ns.engine.node_count();
    let edge_count = ns.engine.graph_store.read().edge_count();
    let schema = ns.engine.property_store.read().schema();
    Json(json!(InfoResp {
        node_count,
        edge_count,
        dimension: ns.dimension,
        property_schema: schema,
    }))
}

async fn metrics(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let total_nodes = ns.engine.node_count();
    let total_edges = ns.engine.graph_store.read().edge_count();
    let (hot, cold) = ns.engine.temporal_store.read().hot_cold_separation(0.3);
    Json(json!(SystemMetrics {
        total_nodes,
        total_edges,
        segment_count: 1,
        hot_memories: hot.len(),
        cold_memories: cold.len(),
    }))
}

async fn add_edge(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<EdgeReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let edge_type = match req.edge_type.as_str() {
        "similar_to" | "SIMILAR_TO" => EdgeType::SimilarTo,
        "derived_from" | "DERIVED_FROM" => EdgeType::DerivedFrom,
        "part_of" | "PART_OF" => EdgeType::PartOf,
        "temporally_after" | "TEMPORALLY_AFTER" => EdgeType::TemporallyAfter,
        "contradicts" | "CONTRADICTS" => EdgeType::Contradicts,
        "references" | "REFERENCES" => EdgeType::References,
        _ => return Json(json!({"error": "unknown edge type"})),
    };
    let edge = Edge {
        from: req.from,
        to: req.to,
        edge_type,
        weight: req.weight.unwrap_or(1.0),
    };
    let result = ns.engine.graph_store.write().add_edge(edge);
    match result {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_edges(
    State(state): State<AppState>,
    Path((ns_name, id)): Path<(String, u64)>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let graph = ns.engine.graph_store.read();
    let edges = graph.get_neighbors(id, None);
    let items: Vec<EdgeItem> = edges.iter().map(|(to, et, w)| EdgeItem {
        from: id,
        to: *to,
        edge_type: format!("{:?}", et),
        weight: *w,
    }).collect();
    Json(json!(items))
}

async fn handle_backup(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<SnapshotReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    match req.action.as_str() {
        "create" => {
            let nodes: Vec<MemoryNode> = ns.engine.node_data.read().values().cloned().collect();
            match ns.backup.create_snapshot(&nodes) {
                Ok(snap) => Json(json!({"ok": true, "snapshot_id": snap.snapshot_id})),
                Err(e) => Json(json!({"error": e.to_string()})),
            }
        }
        "restore" => {
            let sid = match &req.snapshot_id {
                Some(s) => s.clone(),
                None => return Json(json!({"error": "snapshot_id required"})),
            };
            match ns.backup.restore_snapshot(&sid) {
                Ok(nodes) => {
                    for node in nodes {
                        let _ = ns.engine.insert_node(node);
                    }
                    Json(json!({"ok": true}))
                }
                Err(e) => Json(json!({"error": e.to_string()})),
            }
        }
        _ => Json(json!({"error": "unknown action"})),
    }
}

async fn list_backups(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    match ns.backup.list_snapshots() {
        Ok(list) => Json(json!(list)),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn handle_import(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<ImportReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let path = match &req.path {
        Some(p) => p,
        None => return Json(json!({"error": "path required"})),
    };
    let dim = ns.dimension;
    let nodes = match req.format.as_str() {
        "jsonl" => import_export::import_jsonl(path, dim),
        "csv" => import_export::import_csv(path, dim),
        "archive" | "sky" => {
            match import_export::load_archive(path) {
                Ok(archive) => Ok(archive.nodes),
                Err(e) => Err(e),
            }
        }
        _ => return Json(json!({"error": "unsupported format"})),
    };
    match nodes {
        Ok(nodes) => {
            let count = nodes.len();
            for node in nodes {
                let _ = ns.engine.insert_node(node);
            }
            Json(json!({"ok": true, "imported": count}))
        }
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn handle_export(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})).into_response(),
    };
    let nodes: Vec<MemoryNode> = ns.engine.node_data.read().values().cloned().collect();
    let archive = import_export::export_archive(&nodes, ns.dimension);
    let data = serde_json::to_string_pretty(&archive).unwrap_or_default();
    axum::response::Response::builder()
        .header("Content-Type", "application/json")
        .header("Content-Disposition", format!("attachment; filename=\"skymemory_{}_export.json\"", ns_name))
        .body(data)
        .unwrap()
        .into_response()
}

#[derive(Deserialize)]
struct TraverseReq {
    start: u64,
    depth: Option<usize>,
    edge_type: Option<String>,
}

async fn graph_traverse(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<TraverseReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let graph = ns.engine.graph_store.read();
    let et = req.edge_type.as_deref().and_then(|s| match s {
        "similar_to" => Some(EdgeType::SimilarTo),
        "derived_from" => Some(EdgeType::DerivedFrom),
        "part_of" => Some(EdgeType::PartOf),
        "temporally_after" => Some(EdgeType::TemporallyAfter),
        "contradicts" => Some(EdgeType::Contradicts),
        "references" => Some(EdgeType::References),
        _ => None,
    });
    let result = graph.traverse(req.start, req.depth.unwrap_or(2), et);
    Json(json!(result.iter().map(|(id, depth, weight)| {
        json!({"id": id, "depth": depth, "weight": weight})
    }).collect::<Vec<_>>()))
}

#[derive(Deserialize)]
struct ActivateReq {
    start: u64,
    decay_factor: Option<f32>,
    threshold: Option<f32>,
    max_hops: Option<usize>,
}

async fn spreading_activation(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<ActivateReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let graph = ns.engine.graph_store.write();
    let result = graph.spreading_activation(
        req.start,
        req.decay_factor.unwrap_or(0.5),
        req.threshold.unwrap_or(0.1),
        req.max_hops.unwrap_or(3),
    );
    Json(json!(result.iter().map(|(id, activation)| {
        json!({"id": id, "activation": activation})
    }).collect::<Vec<_>>()))
}

// ── Database handlers ───────────────────────────────────────

async fn db_connect(
    State(state): State<AppState>,
    Json(req): Json<DbConnectReq>,
) -> impl IntoResponse {
    let config = DbConfig {
        name: req.name,
        db_type: req.db_type,
        dsn: req.dsn,
        max_connections: req.max_connections,
    };
    match state.db_manager.connect(config).await {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_disconnect(
    State(state): State<AppState>,
    Json(req): Json<DbDisconnectReq>,
) -> impl IntoResponse {
    match state.db_manager.disconnect(&req.name).await {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_list_connections(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let list = state.db_manager.list_connections().await;
    Json(json!(list))
}

async fn db_query(
    State(state): State<AppState>,
    Json(req): Json<DbQueryReq>,
) -> impl IntoResponse {
    match state.db_manager.query(&req.connection, &req.sql).await {
        Ok(result) => Json(json!({"ok": true, "data": result})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_list_tables(
    State(state): State<AppState>,
    Json(req): Json<DbDisconnectReq>,
) -> impl IntoResponse {
    match state.db_manager.list_tables(&req.name).await {
        Ok(tables) => Json(json!({"ok": true, "tables": tables})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_describe_table(
    State(state): State<AppState>,
    Json(req): Json<DbTableReq>,
) -> impl IntoResponse {
    match state.db_manager.describe_table(&req.connection, &req.table).await {
        Ok(result) => Json(json!({"ok": true, "data": result})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_test_connection(
    State(state): State<AppState>,
    Json(req): Json<DbDisconnectReq>,
) -> impl IntoResponse {
    match state.db_manager.test_connection(&req.name).await {
        Ok(alive) => Json(json!({"ok": true, "alive": alive})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn db_import_to_vector(
    State(state): State<AppState>,
    Path(ns_name): Path<String>,
    Json(req): Json<DbImportReq>,
) -> impl IntoResponse {
    let ns = match state.namespaces.get_namespace(&ns_name).await {
        Some(ns) => ns,
        None => return Json(json!({"error": format!("namespace '{}' not found", ns_name)})),
    };
    let query_result = match state.db_manager.query(&req.connection, &req.sql).await {
        Ok(r) => r,
        Err(e) => return Json(json!({"error": format!("query failed: {}", e)})),
    };

    if query_result.rows.is_empty() {
        return Json(json!({"ok": true, "imported": 0, "message": "query returned no rows"}));
    }

    let vec_col_idx = match query_result.columns.iter().position(|c| c == &req.vector_column) {
        Some(idx) => idx,
        None => return Json(json!({"error": format!("vector column '{}' not found in query results. available columns: {:?}", req.vector_column, query_result.columns)})),
    };

    let meta_col_indices: Vec<(usize, String)> = match &req.metadata_columns {
        Some(cols) if !cols.is_empty() => {
            cols.iter().filter_map(|name| {
                query_result.columns.iter().position(|c| c == name)
                    .map(|idx| (idx, name.clone()))
            }).collect()
        }
        _ => {
            query_result.columns.iter().enumerate()
                .filter(|(i, _)| *i != vec_col_idx)
                .map(|(i, name)| (i, name.clone()))
                .collect()
        }
    };

    let mem_type = match req.memory_type.as_deref() {
        Some("episodic") => MemoryType::Episodic,
        _ => MemoryType::Semantic,
    };

    let mut imported = 0usize;
    let mut errors = Vec::new();
    let expected_dim = ns.dimension;

    for (row_idx, row) in query_result.rows.iter().enumerate() {
        let vector = match parse_vector_value(&row[vec_col_idx]) {
            Some(v) => v,
            None => {
                errors.push(format!("row {}: cannot parse vector from column '{}'", row_idx, req.vector_column));
                continue;
            }
        };

        if vector.len() != expected_dim {
            errors.push(format!("row {}: dimension mismatch: expected {}, got {}", row_idx, expected_dim, vector.len()));
            continue;
        }

        let mut properties: Vec<Property> = Vec::new();
        for (col_idx, col_name) in &meta_col_indices {
            if *col_idx < row.len() {
                let val_str = json_value_to_string(&row[*col_idx]);
                properties.push(Property {
                    key: col_name.clone(),
                    value: Value::String(val_str),
                });
            }
        }
        properties.push(Property {
            key: "source".into(),
            value: Value::String("database_import".into()),
        });

        let node = MemoryNode {
            id: generate_node_id(),
            vector,
            properties,
            temporal: TemporalInfo::default(),
            memory_type: mem_type,
        };

        match ns.engine.insert_node(node) {
            Ok(()) => imported += 1,
            Err(e) => errors.push(format!("row {}: insert failed: {}", row_idx, e)),
        }
    }

    let mut resp = json!({"ok": true, "imported": imported, "total_rows": query_result.row_count});
    if !errors.is_empty() {
        resp["errors"] = json!(errors);
    }
    Json(resp)
}

fn parse_vector_value(val: &serde_json::Value) -> Option<Vec<f32>> {
    match val {
        serde_json::Value::Array(arr) => {
            let v: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect();
            if v.len() == arr.len() { Some(v) } else { None }
        }
        serde_json::Value::String(s) => {
            let s = s.trim();
            if s.starts_with('[') {
                if let Ok(arr) = serde_json::from_str::<Vec<f32>>(s) {
                    return Some(arr);
                }
            }
            let v: Vec<f32> = s.split(',')
                .map(|x| x.trim().parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            if v.is_empty() { None } else { Some(v) }
        }
        _ => None,
    }
}

fn json_value_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
