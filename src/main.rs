use clap::{Parser, Subcommand};
use skymemory_core::{Edge, EdgeType, MemoryNode, MemoryType, Metric, Property, TemporalInfo, Value, generate_node_id, now_millis};
use skymemory_storage::SegmentManager;
use skymemory_query::QueryEngine;
use skymemory_query::parse_query;
use skymemory_backup::BackupManager;
use skymemory_import_export as import_export;
use skymemory_api::NamespaceManager;

#[derive(Parser)]
#[command(name = "skymemory")]
#[command(about = "SkyMemory - Cognitive Memory Engine", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "data")]
    dir: String,

    #[arg(long, default_value_t = 128)]
    dimension: usize,

    #[arg(long)]
    hf_mirror: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
    },
    Insert {
        vector: Vec<f32>,
        #[arg(short, long, num_args = 0.., value_delimiter = ',')]
        meta: Vec<String>,
        #[arg(long, default_value = "semantic")]
        memory_type: String,
    },
    Search {
        vector: Vec<f32>,
        #[arg(short, long, default_value_t = 10)]
        k: usize,
        #[arg(short, long, default_value = "cs")]
        metric: String,
    },
    Query {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        k: usize,
    },
    Edge {
        from: u64,
        to: u64,
        #[arg(long)]
        edge_type: String,
        #[arg(long, default_value_t = 1.0)]
        weight: f32,
    },
    Info,
    Export {
        #[arg(short, long, default_value = "export.json")]
        output: String,
    },
    Import {
        #[arg(short, long)]
        path: String,
        #[arg(long, default_value = "jsonl")]
        format: String,
    },
    Backup,
    Restore {
        snapshot_id: String,
    },
    ListBackups,
}

fn parse_meta(pairs: Vec<String>) -> Vec<Property> {
    pairs.into_iter().filter_map(|p| {
        let mut it = p.splitn(2, '=');
        let k = it.next()?.to_string();
        let v = it.next().unwrap_or("").to_string();
        Some(Property { key: k, value: Value::String(v) })
    }).collect()
}

fn parse_edge_type(s: &str) -> EdgeType {
    match s.to_lowercase().as_str() {
        "similar_to" => EdgeType::SimilarTo,
        "derived_from" => EdgeType::DerivedFrom,
        "part_of" => EdgeType::PartOf,
        "temporally_after" => EdgeType::TemporallyAfter,
        "contradicts" => EdgeType::Contradicts,
        "references" => EdgeType::References,
        _ => EdgeType::References,
    }
}

fn build_engine(data_dir: &str, dimension: usize) -> anyhow::Result<(QueryEngine, SegmentManager)> {
    let seg_mgr = SegmentManager::open(data_dir)?;
    let engine = QueryEngine::new(dimension);
    for node in seg_mgr.snapshot()? {
        let _ = engine.insert_node(node);
    }
    Ok((engine, seg_mgr))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { addr } => {
            let models_dir = std::path::PathBuf::from(&cli.dir).join("models");
            let namespaces = NamespaceManager::new(&cli.dir, cli.dimension)?;
            let app = skymemory_api::build_router(namespaces, cli.dimension, models_dir, cli.hf_mirror).await;
            println!("SkyMemory listening on http://{}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        Commands::Insert { vector, meta, memory_type } => {
            if vector.len() != cli.dimension {
                eprintln!("dimension mismatch: expected {}, got {}", cli.dimension, vector.len());
                std::process::exit(1);
            }
            let mut seg_mgr = SegmentManager::open(&cli.dir)?;
            let mut properties = parse_meta(meta);
            properties.push(Property {
                key: "created_at".into(),
                value: Value::Timestamp(now_millis()),
            });
            let mem_type = match memory_type.as_str() {
                "episodic" => MemoryType::Episodic,
                _ => MemoryType::Semantic,
            };
            let node = MemoryNode {
                id: generate_node_id(),
                vector,
                properties,
                temporal: TemporalInfo::default(),
                memory_type: mem_type,
            };
            println!("inserted id={}", node.id);
            seg_mgr.insert(node)?;
            seg_mgr.flush_all()?;
        }
        Commands::Search { vector, k, metric } => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let m = Metric::from_code(&metric).ok_or("unknown metric")?;
            let results = engine.vector_store.search(&vector, k, &m)?;
            for (i, (id, dist)) in results.iter().enumerate() {
                let node = engine.get_node(*id);
                let meta_str = node
                    .as_ref()
                    .map(|n| n.properties.iter().map(|p| format!("{}={}", p.key, p.value)).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                println!("{}\tid={}\tdist={:.6}\t{}", i, id, dist, meta_str);
            }
        }
        Commands::Query { query, k } => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let parsed = parse_query(&query)?;
            let results = engine.execute(&parsed, k, &Metric::Cosine)?;
            for (i, r) in results.iter().enumerate() {
                let meta_str = r.properties.iter().map(|p| format!("{}={}", p.key, p.value)).collect::<Vec<_>>().join(", ");
                println!("{}\tid={}\tscore={:.4}\tdist={:.6}\ttype={:?}\t{}", i, r.node_id, r.score, r.distance, r.memory_type, meta_str);
            }
        }
        Commands::Edge { from, to, edge_type, weight } => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let edge = Edge { from, to, edge_type: parse_edge_type(&edge_type), weight };
            engine.graph_store.write().add_edge(edge)?;
            println!("edge added: {} -> {} ({})", from, to, edge_type);
        }
        Commands::Info => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let node_count = engine.node_count();
            let edge_count = engine.graph_store.read().edge_count();
            let schema = engine.property_store.read().schema();
            println!("nodes={}", node_count);
            println!("edges={}", edge_count);
            println!("dimension={}", cli.dimension);
            for (k, types) in &schema {
                println!("property {}: {:?}", k, types);
            }
        }
        Commands::Export { output } => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let nodes: Vec<MemoryNode> = engine.node_data.read().values().cloned().collect();
            let archive = import_export::export_archive(&nodes, cli.dimension);
            import_export::save_archive(&archive, &output)?;
            println!("exported {} nodes to {}", archive.manifest.node_count, output);
        }
        Commands::Import { path, format } => {
            let nodes = match format.as_str() {
                "jsonl" => import_export::import_jsonl(&path, cli.dimension)?,
                "csv" => import_export::import_csv(&path, cli.dimension)?,
                "archive" | "sky" => import_export::load_archive(&path)?.nodes,
                _ => {
                    eprintln!("unsupported format: {}", format);
                    std::process::exit(1);
                }
            };
            let count = nodes.len();
            let mut seg_mgr = SegmentManager::open(&cli.dir)?;
            for node in nodes {
                seg_mgr.insert(node)?;
            }
            seg_mgr.flush_all()?;
            println!("imported {} nodes", count);
        }
        Commands::Backup => {
            let (engine, _) = build_engine(&cli.dir, cli.dimension)?;
            let nodes: Vec<MemoryNode> = engine.node_data.read().values().cloned().collect();
            let backup_mgr = BackupManager::new(&cli.dir);
            let snapshot = backup_mgr.create_snapshot(&nodes)?;
            println!("backup created: {} ({} nodes)", snapshot.snapshot_id, snapshot.node_count);
        }
        Commands::Restore { snapshot_id } => {
            let backup_mgr = BackupManager::new(&cli.dir);
            let nodes = backup_mgr.restore_snapshot(&snapshot_id)?;
            let count = nodes.len();
            let mut seg_mgr = SegmentManager::open(&cli.dir)?;
            for node in nodes {
                seg_mgr.insert(node)?;
            }
            seg_mgr.flush_all()?;
            println!("restored {} nodes from {}", count, snapshot_id);
        }
        Commands::ListBackups => {
            let backup_mgr = BackupManager::new(&cli.dir);
            let snapshots = backup_mgr.list_snapshots()?;
            if snapshots.is_empty() {
                println!("no backups found");
            }
            for s in &snapshots {
                println!("{}", s);
            }
        }
    }

    Ok(())
}
