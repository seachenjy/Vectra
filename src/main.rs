mod ver;
use clap::{Parser, Subcommand};
use chrono::Utc;
use ver::{Vector, MetadataEntry, MetadataValue, Database, Metric, distance};
use std::sync::{Arc, Mutex};
use axum::{extract::{Path, State}, routing::post, Json, Router};
use serde::{Serialize, Deserialize};

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
    Serve { #[arg(short = 'a', long = "addr", default_value = "127.0.0.1:8080")] addr: String },
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
struct AppState { dir: String, dbs: Arc<Mutex<std::collections::HashMap<String, Database>>> }

#[derive(Deserialize)]
struct CreateReq { name: String, dimension: usize }

#[derive(Deserialize)]
struct InsertReq { values: Vec<f64>, meta: std::collections::HashMap<String, String> }

#[derive(Deserialize)]
struct FindReq { values: Vec<f64>, k: Option<usize>, f: Option<String> }

#[derive(Serialize)]
struct FindItem { index: usize, distance: f64, values: Vec<f64>, metadata: std::collections::HashMap<String, String> }

async fn create_db(State(state): State<AppState>, Json(req): Json<CreateReq>) -> Result<Json<serde_json::Value>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    if map.contains_key(&req.name) { return Err("exists".into()); }
    let db = Database::new(req.name.clone(), req.dimension);
    db.save_to_dir(&state.dir).map_err(|e| e.to_string())?;
    map.insert(req.name.clone(), db);
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn insert_vec(State(state): State<AppState>, Path(name): Path<String>, Json(req): Json<InsertReq>) -> Result<Json<serde_json::Value>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    let db = map.entry(name.clone()).or_insert_with(|| Database::load_from_dir(&state.dir, &name).unwrap_or(Database::new(name.clone(), req.values.len())));
    if db.dimension != req.values.len() { return Err(format!("dimension mismatch: db={}, input={}", db.dimension, req.values.len())); }
    let mut meta = Vec::new();
    for (k,v) in req.meta.iter() { meta.push(MetadataEntry::new(k.clone(), MetadataValue::String(v.clone()))); }
    meta.push(MetadataEntry::new("created_at".to_string(), MetadataValue::DateTime(Utc::now())));
    let v = Vector::new(req.values, meta);
    db.insert(v).map_err(|e| e.to_string())?;
    db.save_to_dir(&state.dir).map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"ok": true, "total": db.vectors.len()})))
}

async fn find_vec(State(state): State<AppState>, Path(name): Path<String>, Json(req): Json<FindReq>) -> Result<Json<Vec<FindItem>>, String> {
    let mut map = state.dbs.lock().map_err(|_| "lock")?;
    if !map.contains_key(&name) {
        let db = Database::load_from_dir(&state.dir, &name).map_err(|_| "not found")?;
        map.insert(name.clone(), db);
    }
    let db = map.get(&name).unwrap();
    if db.dimension != req.values.len() { return Err(format!("dimension mismatch: db={}, input={}", db.dimension, req.values.len())); }
    let metric = Metric::from_code(req.f.as_deref().unwrap_or("eu")).ok_or("unknown metric")?;
    let mut scored: Vec<(usize, f64)> = db.vectors.iter().enumerate()
        .map(|(i, v)| (i, distance(v.data(), &req.values, &metric)))
        .collect();
    scored.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
    let k = req.k.unwrap_or(10);
    let mut res = Vec::new();
    for (idx, dist) in scored.into_iter().take(k) {
        let mut meta_map = std::collections::HashMap::new();
        for m in db.vectors[idx].metadata() { meta_map.insert(m.key().to_string(), m.value().to_string()); }
        let values = db.vectors[idx].data().to_vec();
        res.push(FindItem { index: idx, distance: dist, values, metadata: meta_map });
    }
    Ok(Json(res))
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
        Commands::Serve { addr } => {
            let state = AppState { dir: cli.dir.clone(), dbs: Arc::new(Mutex::new(std::collections::HashMap::new())) };
            let app = Router::new()
                .route("/create", post(create_db))
                .route("/db/:name/insert", post(insert_vec))
                .route("/db/:name/find", post(find_vec))
                .with_state(state);
            println!("listening on http://{}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}