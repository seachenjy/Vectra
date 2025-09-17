mod ver;
use clap::{Parser, Subcommand};
use chrono::Utc;
use ver::{Vector, MetadataEntry, MetadataValue, Database, Metric, distance};
use std::sync::{Arc, Mutex};
use axum::{extract::{Path, State}, routing::{post, get}, Json, Router};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use rusqlite::{Connection, types::ValueRef};
use std::fs;

#[derive(Parser)]
#[command(name = "vectra")]
#[command(about = "Simple local vector engine", long_about = None)]
struct Cli {
    /// Data directory
    #[arg(short, long, default_value = "data")] 
    dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new database
    Create { name: String, #[arg(short = 'd')] dimension: usize },

    /// Insert a vector into a database with optional metadata key=value pairs
    Insert { name: String, #[arg(short = 'v', num_args = 1.., value_delimiter = ',')] values: Vec<f64>, #[arg(short = 'm', num_args = 0.., value_delimiter = ',')] meta: Vec<String> },

    /// Find nearest vectors in a database
    Find { name: String, #[arg(short = 'v', num_args = 1.., value_delimiter = ',')] values: Vec<f64>, #[arg(short = 'k', default_value_t = 10)] k: usize, #[arg(short = 'f', default_value = "eu")] f: String },

    /// Serve REST API
    Serve { #[arg(short = 'a', long = "addr", default_value = "127.0.0.1:8080")] addr: String,
            #[arg(long = "cache-max-mb", default_value_t = 128)] cache_max_mb: usize,
            #[arg(long = "flush-interval-sec", default_value_t = 5)] flush_interval_sec: u64,
            #[arg(long = "cache-ttl-sec", default_value_t = 600)] cache_ttl_sec: u64 },

    /// Import from SQLite table
    ImportSqlite {
        /// SQLite database file path
        #[arg(long)] sqlite: String,
        /// Table name
        #[arg(long)] table: String,
        /// Target vectra db name
        #[arg(long)] name: String,
        /// Vector columns (comma-separated)
        #[arg(long, value_delimiter = ',')] vec_cols: Vec<String>,
        /// Metadata columns (comma-separated key=value where value is column name, e.g., source=src_col)
        #[arg(long, value_delimiter = ',')] meta_cols: Vec<String>,
        /// Batch size (rows per shard file)
        #[arg(long, default_value_t = 200_000)] batch_size: usize,
    },

    /// Show DB info (dimension, count, metadata schema)
    Info { name: String },
}

fn parse_meta(pairs: Vec<String>) -> Vec<MetadataEntry> {
    pairs.into_iter().filter_map(|p| {
        let mut it = p.splitn(2, '=');
        let k = it.next()?;
        let v = it.next().unwrap_or("");
        Some(MetadataEntry::new(k.to_string(), MetadataValue::String(v.to_string())))
    }).collect()
}

#[derive(Clone)]
struct AppState { dir: String, dbs: Arc<Mutex<std::collections::HashMap<String, CacheEntry>>>,
                  cache_max_bytes: usize, flush_interval: Duration, cache_ttl: Duration }

struct CacheEntry { db: Database, last_access: Instant, dirty: bool }

fn evict_if_needed(map: &mut std::collections::HashMap<String, CacheEntry>, max_bytes: usize, ttl: Duration) {
    // TTL eviction
    let now = Instant::now();
    let keys_to_remove: Vec<String> = map.iter()
        .filter(|(_, e)| now.duration_since(e.last_access) > ttl && !e.dirty)
        .map(|(k, _)| k.clone())
        .collect();
    for k in keys_to_remove { map.remove(&k); }

    // Byte-size based eviction: estimate bytes and evict clean & oldest first
    let mut total_bytes: usize = map.values().map(estimate_entry_bytes).sum();
    if total_bytes <= max_bytes { return; }
    let mut entries: Vec<(String, Instant, bool, usize)> = map.iter().map(|(k, e)| (k.clone(), e.last_access, e.dirty, estimate_entry_bytes(e))).collect();
    entries.sort_by_key(|(_, last, dirty, _)| (*dirty, *last));
    while total_bytes > max_bytes {
        if let Some((k, _, _, sz)) = entries.first().cloned() { map.remove(&k); total_bytes = total_bytes.saturating_sub(sz); entries.remove(0); } else { break; }
    }
}

fn estimate_entry_bytes(e: &CacheEntry) -> usize {
    // rough estimate: vectors values + metadata strings
    let mut bytes = 0usize;
    for v in &e.db.vectors {
        bytes = bytes.saturating_add(v.data().len() * std::mem::size_of::<f64>());
        for m in v.metadata() {
            bytes = bytes.saturating_add(m.key().len());
            // value rough size
            bytes = bytes.saturating_add(32);
        }
    }
    bytes
}

fn map_value_ref_to_metadata(v: ValueRef<'_>) -> Option<MetadataValue> {
    match v {
        ValueRef::Integer(n) => Some(MetadataValue::Integer(n as i32)),
        ValueRef::Real(r) => Some(MetadataValue::Float(r as f32)),
        ValueRef::Text(t) => {
            let s = std::str::from_utf8(t).ok()?.to_string();
            // bool
            if s.eq_ignore_ascii_case("true") || s == "1" { return Some(MetadataValue::Bool(true)); }
            if s.eq_ignore_ascii_case("false") || s == "0" { return Some(MetadataValue::Bool(false)); }
            // datetime
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) { return Some(MetadataValue::DateTime(dt.with_timezone(&Utc))); }
            // fallback string
            Some(MetadataValue::String(s))
        }
        _ => None,
    }
}

#[derive(Deserialize)]
struct CreateReq { name: String, dimension: usize }

#[derive(Deserialize)]
struct InsertReq { values: Vec<f64>, meta: std::collections::HashMap<String, String> }

#[derive(Deserialize)]
struct FindReq { values: Vec<f64>, k: Option<usize>, f: Option<String> }

#[derive(Serialize)]
struct FindItem { index: usize, distance: f64, values: Vec<f64>, metadata: std::collections::HashMap<String, String> }

#[derive(Serialize)]
struct InfoResp { name: String, dimension: usize, count: usize, metadata_schema: std::collections::HashMap<String, Vec<String>> }

async fn create_db(State(state): State<AppState>, Json(req): Json<CreateReq>) -> Result<Json<serde_json::Value>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    if map.contains_key(&req.name) { return Err("exists".into()); }
    let db = Database::new(req.name.clone(), req.dimension);
    db.save_to_dir(&state.dir).map_err(|e| e.to_string())?;
    map.insert(req.name.clone(), CacheEntry { db, last_access: Instant::now(), dirty: false });
    evict_if_needed(&mut map, state.cache_max_bytes, state.cache_ttl);
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn insert_vec(State(state): State<AppState>, Path(name): Path<String>, Json(req): Json<InsertReq>) -> Result<Json<serde_json::Value>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    let entry = map.entry(name.clone()).or_insert_with(|| {
        let db = Database::load_from_dir(&state.dir, &name).unwrap_or(Database::new(name.clone(), req.values.len()));
        CacheEntry { db, last_access: Instant::now(), dirty: false }
    });
    if entry.db.dimension != req.values.len() { return Err(format!("dimension mismatch: db={}, input={}", entry.db.dimension, req.values.len())); }
    let mut meta = Vec::new();
    for (k,v) in req.meta.iter() { meta.push(MetadataEntry::new(k.clone(), MetadataValue::String(v.clone()))); }
    meta.push(MetadataEntry::new("created_at".to_string(), MetadataValue::DateTime(Utc::now())));
    let v = Vector::new(req.values, meta);
    entry.db.insert(v).map_err(|e| e.to_string())?;
    entry.dirty = true;
    entry.last_access = Instant::now();
    let total = entry.db.vectors.len();
    let _ = entry;
    evict_if_needed(&mut map, state.cache_max_bytes, state.cache_ttl);
    Ok(Json(serde_json::json!({"ok": true, "total": total})))
}

async fn find_vec(State(state): State<AppState>, Path(name): Path<String>, Json(req): Json<FindReq>) -> Result<Json<Vec<FindItem>>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    if !map.contains_key(&name) {
        let db = Database::load_from_dir(&state.dir, &name).map_err(|_| "not found")?;
        map.insert(name.clone(), CacheEntry { db, last_access: Instant::now(), dirty: false });
    }
    let entry = map.get_mut(&name).unwrap();
    if entry.db.dimension != req.values.len() { return Err(format!("dimension mismatch: db={}, input={}", entry.db.dimension, req.values.len())); }
    let metric = Metric::from_code(req.f.as_deref().unwrap_or("eu")).ok_or("unknown metric")?;
    let mut scored: Vec<(usize, f64)> = entry.db.vectors.iter().enumerate()
        .map(|(i, v)| (i, distance(v.data(), &req.values, &metric)))
        .collect();
    scored.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
    let k = req.k.unwrap_or(10);
    let mut res = Vec::new();
    for (idx, dist) in scored.into_iter().take(k) {
        let mut meta_map = std::collections::HashMap::new();
        for m in entry.db.vectors[idx].metadata() { meta_map.insert(m.key().to_string(), m.value().to_string()); }
        let values = entry.db.vectors[idx].data().to_vec();
        res.push(FindItem { index: idx, distance: dist, values, metadata: meta_map });
    }
    entry.last_access = Instant::now();
    evict_if_needed(&mut map, state.cache_max_bytes, state.cache_ttl);
    Ok(Json(res))
}

fn metadata_type_name(v: &MetadataValue) -> &'static str {
    match v {
        MetadataValue::Integer(_) => "Integer",
        MetadataValue::Float(_) => "Float",
        MetadataValue::String(_) => "String",
        MetadataValue::Bool(_) => "Bool",
        MetadataValue::DateTime(_) => "DateTime",
    }
}

fn build_metadata_schema(db: &Database) -> std::collections::HashMap<String, Vec<String>> {
    use std::collections::{HashMap, HashSet};
    let mut m: HashMap<String, HashSet<&'static str>> = HashMap::new();
    for v in &db.vectors {
        for e in v.metadata() { m.entry(e.key().to_string()).or_default().insert(metadata_type_name(e.value())); }
    }
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (k, set) in m { let mut v: Vec<String> = set.into_iter().map(|s| s.to_string()).collect(); v.sort(); out.insert(k, v); }
    out
}

async fn info_db(State(state): State<AppState>, Path(name): Path<String>) -> Result<Json<InfoResp>, String> {
    let info = compute_db_info(&state.dir, &name).map_err(|e| e.to_string())?;
    Ok(Json(info))
}

fn compute_db_info(dir: &str, name: &str) -> std::io::Result<InfoResp> {
    use std::collections::{HashMap, HashSet};
    let mut dimension: usize = 0;
    let mut count: usize = 0;
    let mut schema: HashMap<String, HashSet<&'static str>> = HashMap::new();

    let mut consider_path = |path: &std::path::Path| -> std::io::Result<()> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut buffer)?;
        let db: Database = bincode::deserialize(&buffer).unwrap();
        if dimension == 0 { dimension = db.dimension; }
        if db.dimension != dimension { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "dimension mismatch in shards")); }
        count += db.vectors.len();
        for v in db.vectors.iter() {
            for e in v.metadata() { schema.entry(e.key().to_string()).or_default().insert(metadata_type_name(e.value())); }
        }
        Ok(())
    };

    let primary_path = std::path::Path::new(dir).join(format!("{}.bin", name));
    if primary_path.exists() { consider_path(&primary_path)?; }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if fname.starts_with(&format!("{}_part_", name)) && fname.ends_with(".bin") {
                    consider_path(&path)?;
                }
            }
        }
    }
    if dimension == 0 { return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "database not found")); }
    let mut schema_out: HashMap<String, Vec<String>> = HashMap::new();
    for (k, set) in schema { let mut v: Vec<String> = set.into_iter().map(|s| s.to_string()).collect(); v.sort(); schema_out.insert(k, v); }
    Ok(InfoResp { name: name.to_string(), dimension, count, metadata_schema: schema_out })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let cli = Cli::parse();
    match cli.command {
        Commands::Create { name, dimension } => {
            let db = Database::new(name.clone(), dimension);
            db.save_to_dir(&cli.dir)?;
            println!("created db '{}' with dimension {} in {}", name, dimension, cli.dir);
        }
        Commands::Insert { name, values, meta } => {
            let mut db = Database::load_from_dir(&cli.dir, &name)?;
            if db.dimension != values.len() { eprintln!("dimension mismatch: db={}, input={}", db.dimension, values.len()); std::process::exit(1); }
            let mut m = parse_meta(meta);
            m.push(MetadataEntry::new("created_at".to_string(), MetadataValue::DateTime(Utc::now())));
            let v = Vector::new(values, m);
            db.insert(v)?;
            println!("ok: inserted one vector (total={})", db.vectors.len());
            db.save_to_dir(&cli.dir)?;
            println!("inserted into '{}' (total={})", name, db.vectors.len());
        }
        Commands::Find { name, values, k, f } => {
            let db = Database::load_from_dir(&cli.dir, &name)?;
            if db.dimension != values.len() { eprintln!("dimension mismatch: db={}, input={}", db.dimension, values.len()); std::process::exit(1); }
            let metric = Metric::from_code(&f).ok_or("unknown metric code")?;
            let mut scored: Vec<(usize, f64)> = db.vectors.iter().enumerate()
                .map(|(i, v)| (i, distance(v.data(), &values, &metric)))
                .collect();
            scored.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
            for (i,(idx, dist)) in scored.into_iter().take(k).enumerate() {
                let v = &db.vectors[idx];
                let src = v.metadata().iter().find(|m| m.key() == "source").map(|m| m.value().to_string()).unwrap_or_else(|| "".to_string());
                println!("{}\tidx={}\tdist={:.6}\tsource={}\tvalues={:?}", i, idx, dist, src, v.data());
            }
        }
        Commands::Serve { addr, cache_max_mb, flush_interval_sec, cache_ttl_sec } => {
            let state = AppState { dir: cli.dir.clone(), dbs: Arc::new(Mutex::new(std::collections::HashMap::new())), cache_max_bytes: cache_max_mb * 1024 * 1024, flush_interval: Duration::from_secs(flush_interval_sec), cache_ttl: Duration::from_secs(cache_ttl_sec) };
            // background flush task
            let state_clone = state.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(state_clone.flush_interval).await;
                    let mut map = match state_clone.dbs.lock() { Ok(g) => g, Err(_) => continue };
                    let keys: Vec<String> = map.keys().cloned().collect();
                    for k in keys {
                        if let Some(entry) = map.get_mut(&k) {
                            if entry.dirty {
                                if entry.db.save_to_dir(&state_clone.dir).is_ok() {
                                    entry.dirty = false;
                                }
                            }
                        }
                    }
                    evict_if_needed(&mut map, state_clone.cache_max_bytes, state_clone.cache_ttl);
                }
            });
            let app = Router::new()
                .route("/create", post(create_db))
                .route("/db/:name/insert", post(insert_vec))
                .route("/db/:name/find", post(find_vec))
                .route("/db/:name/info", get(info_db))
                .with_state(state);
            println!("listening on http://{}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        Commands::ImportSqlite { sqlite, table, name, vec_cols, meta_cols, batch_size } => {
            let mut conn = Connection::open(sqlite)?;
            // Prepare columns
            let vec_cols_trim: Vec<String> = vec_cols.into_iter().map(|c| c.trim().to_string()).collect();
            let mut meta_map: Vec<(String, String)> = Vec::new();
            for kv in meta_cols { if let Some((k,v)) = kv.split_once('=') { meta_map.push((k.to_string(), v.to_string())); } }
            // Build SQL with quoted idents
            fn quote_ident(s: &str) -> String {
                let escaped = s.replace('"', "\"\"");
                format!("\"{}\"", escaped)
            }
            let select_cols: Vec<String> = vec_cols_trim.iter().map(|c| quote_ident(c)).chain(meta_map.iter().map(|(_,v)| quote_ident(v))).collect();
            let sql = format!("SELECT {} FROM {}", select_cols.join(","), quote_ident(&table));
            // Prepare DB shard buffer
            let mut db = Database::new(name.clone(), vec_cols_trim.len());
            let mut shard_index: usize = 0;
            // Stream rows, robust parsing
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare(&sql)?;
                let mut rows = stmt.query([])?;
                let mut count: usize = 0;
                let mut skipped: usize = 0;
                while let Some(row) = rows.next()? {
                    let mut values: Vec<f64> = Vec::with_capacity(vec_cols_trim.len());
                    let mut ok = true;
                    for i in 0..vec_cols_trim.len() {
                        match row.get_ref(i)? {
                            ValueRef::Real(r) => values.push(r),
                            ValueRef::Integer(n) => values.push(n as f64),
                            ValueRef::Text(t) => {
                                if let Ok(s) = std::str::from_utf8(t) { if let Ok(f) = s.trim().parse::<f64>() { values.push(f); } else { ok = false; break; } } else { ok = false; break; }
                            }
                            _ => { ok = false; break; }
                        }
                    }
                    if !ok || values.len() != vec_cols_trim.len() { skipped += 1; continue; }
                    let mut metas = Vec::new();
                    for (j, (mk, _)) in meta_map.iter().enumerate() {
                        let idx = vec_cols_trim.len() + j;
                        if let Ok(vr) = row.get_ref(idx) { if let Some(mv) = map_value_ref_to_metadata(vr) { metas.push(MetadataEntry::new(mk.clone(), mv)); } }
                    }
                    metas.push(MetadataEntry::new("created_at".to_string(), MetadataValue::DateTime(Utc::now())));
                if db.dimension != values.len() { skipped += 1; continue; }
                if let Err(e) = db.insert(Vector::new(values, metas)) { eprintln!("skip row due to insert error: {}", e); skipped += 1; continue; }
                    count += 1;
                if db.vectors.len() >= batch_size {
                    let shard_path = format!("{}/{}_part_{}.bin", &cli.dir, &name, shard_index);
                    db.save_to_path(&shard_path).map_err(|e| format!("failed to save shard: {}", e)).unwrap();
                    db.vectors.clear();
                    shard_index += 1;
                }
                    if count % 1000 == 0 { println!("progress: imported {} rows (skipped {})", count, skipped); }
                }
            // Save remaining shard
            if !db.vectors.is_empty() {
                let shard_path = format!("{}/{}_part_{}.bin", &cli.dir, &name, shard_index);
                db.save_to_path(&shard_path).map_err(|e| format!("failed to save shard: {}", e)).unwrap();
            }
            println!("imported {} rows into '{}' (skipped {}), shards={}", count, name, skipped, shard_index + 1);
            }
            tx.commit()?;
            
        }
        Commands::Info { name } => {
            let info = compute_db_info(&cli.dir, &name)?;
            println!("name={} dimension={} count={}", info.name, info.dimension, info.count);
            for (k, types) in info.metadata_schema { println!("meta {}: {:?}", k, types); }
        }
    }
    Ok(())
}