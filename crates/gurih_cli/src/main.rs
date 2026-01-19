use clap::{Parser, Subcommand};
use std::path::PathBuf;
use gurih_dsl::compile;
use std::fs;
use gurih_runtime::data::DataEngine;
use gurih_runtime::storage::MemoryStorage;
use gurih_runtime::context::RuntimeContext;
use std::sync::Arc;
use tokio;
use axum::{
    routing::{get, post},
    Router,
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "gurih")]
#[command(about = "GurihERP CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the DSL file for errors
    Check {
        file: PathBuf,
    },
    /// Compile DSL to IR JSON
    Build {
        file: PathBuf,
        #[arg(short, long, default_value = "schema.json")]
        output: PathBuf,
    },
    /// Run the runtime with the given DSL file (in-memory)
    Run {
        file: PathBuf,
        #[arg(long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Clone)]
struct AppState {
    data_engine: Arc<DataEngine>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => {
            match read_and_compile(&file) {
                Ok(_) => println!("✔ Schema is valid."),
                Err(e) => eprintln!("❌ Error: {}", e),
            }
        }
        Commands::Build { file, output } => {
            match read_and_compile(&file) {
                Ok(schema) => {
                    let json = serde_json::to_string_pretty(&schema).unwrap();
                    fs::write(output, json).expect("Failed to write output file");
                    println!("✔ Build successful.");
                }
                Err(e) => eprintln!("❌ Error: {}", e),
            }
        }
        Commands::Run { file, port } => {
            match read_and_compile(&file) {
                Ok(schema) => {
                    println!("✔ Schema loaded. Starting runtime...");
                    let schema = Arc::new(schema);
                    let storage = Arc::new(MemoryStorage::new());
                    let engine = Arc::new(DataEngine::new(schema.clone(), storage));

                    println!("Runtime initialized with {} entities.", schema.entities.len());
                    
                    let state = AppState { data_engine: engine };
                    
                    let app = Router::new()
                        .route("/api/{entity}", post(create_entity).get(list_entities))
                        .route("/api/{entity}/{id}", get(get_entity).put(update_entity).delete(delete_entity))
                        .with_state(state);

                    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
                    println!("🚀 Server running on http://0.0.0.0:{}", port);
                    axum::serve(listener, app).await.unwrap();
                }
                Err(e) => eprintln!("❌ Error: {}", e),
            }
        }
    }
}

fn read_and_compile(path: &PathBuf) -> Result<gurih_ir::Schema, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    compile(&content).map_err(|e| format!("{:?}", e))
}

// Handlers

async fn create_entity(
    State(state): State<AppState>,
    Path(entity): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let ctx = RuntimeContext::system(); // Using system context for now (TODO: Extract from Auth header)
    match state.data_engine.create(&entity, payload, &ctx).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn list_entities(
    State(state): State<AppState>,
    Path(entity): Path<String>,
) -> impl IntoResponse {
    match state.data_engine.list(&entity).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn get_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.data_engine.read(&entity, &id).await {
        Ok(Some(item)) => (StatusCode::OK, Json(item)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Not found" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn update_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let ctx = RuntimeContext::system();
    match state.data_engine.update(&entity, &id, payload, &ctx).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "updated" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn delete_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.data_engine.delete(&entity, &id).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "deleted" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}
