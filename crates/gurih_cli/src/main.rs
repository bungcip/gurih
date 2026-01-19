use axum::{
    Router,
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use clap::{Parser, Subcommand};
use gurih_dsl::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::persistence::SchemaManager;
use gurih_runtime::storage::{MemoryStorage, PostgresStorage, Storage};
use gurih_runtime::{form::FormEngine, page::PageEngine, portal::PortalEngine};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

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
    Check { file: PathBuf },
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
        /// Only run the backend server, skip frontend and browser
        #[arg(long)]
        server_only: bool,
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
        Commands::Check { file } => match read_and_compile(&file) {
            Ok(_) => println!("✔ Schema is valid."),
            Err(e) => eprintln!("❌ Error: {}", e),
        },
        Commands::Build { file, output } => match read_and_compile(&file) {
            Ok(schema) => {
                let json = serde_json::to_string_pretty(&schema).unwrap();
                fs::write(output, json).expect("Failed to write output file");
                println!("✔ Build successful.");
            }
            Err(e) => eprintln!("❌ Error: {}", e),
        },
        Commands::Run {
            file,
            port,
            server_only,
        } => {
            match read_and_compile(&file) {
                Ok(schema) => {
                    println!("✔ Schema loaded. Starting runtime...");
                    let schema = Arc::new(schema);

                    // Initialize Storage
                    let storage: Arc<dyn Storage> = if let Some(db_config) = &schema.database {
                        println!("🔌 Connecting to database...");
                        // Handle env:DATABASE_URL
                        let url = if db_config.url.starts_with("env:") {
                            std::env::var(&db_config.url[4..]).unwrap_or_else(|_| "".to_string())
                        } else {
                            db_config.url.clone()
                        };

                        if url.is_empty() {
                            panic!("Database URL is empty or env var not set.");
                        }

                        let pool = PgPoolOptions::new()
                            .max_connections(5)
                            .connect(&url)
                            .await
                            .expect("Failed to connect to DB");

                        let manager = SchemaManager::new(pool.clone(), schema.clone());
                        manager.migrate().await.expect("Migration failed");

                        Arc::new(PostgresStorage::new(pool))
                    } else {
                        println!("⚠️ No database configured. Using in-memory storage.");
                        Arc::new(MemoryStorage::new())
                    };

                    let engine = Arc::new(DataEngine::new(schema.clone(), storage));

                    println!(
                        "Runtime initialized with {} entities.",
                        schema.entities.len()
                    );

                    let mut frontend_child = None;

                    if !server_only {
                        // Start Frontend if exists
                        let frontend_port = 5173;
                        let web_path = std::path::Path::new("web");
                        if web_path.exists() {
                            println!("🚀 Starting frontend in web directory...");
                            #[cfg(windows)]
                            let npm_cmd = "npm.cmd";
                            #[cfg(not(windows))]
                            let npm_cmd = "npm";

                            match tokio::process::Command::new(npm_cmd)
                                .arg("run")
                                .arg("dev")
                                .current_dir(web_path)
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn()
                            {
                                Ok(child) => frontend_child = Some(child),
                                Err(e) => eprintln!("⚠️ Failed to start frontend: {}", e),
                            }
                        }

                        // Open Browser
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            println!("🌐 Opening dashboard at http://localhost:{}", frontend_port);
                            #[cfg(windows)]
                            let _ = std::process::Command::new("cmd")
                                .args([
                                    "/C",
                                    "start",
                                    &format!("http://localhost:{}", frontend_port),
                                ])
                                .spawn();
                            #[cfg(not(windows))]
                            let _ = std::process::Command::new("open")
                                .arg(format!("http://localhost:{}", frontend_port))
                                .spawn();
                        });
                    }

                    let state = AppState {
                        data_engine: engine,
                    };

                    let app = Router::new()
                        .route("/api/{entity}", post(create_entity).get(list_entities))
                        .route(
                            "/api/{entity}/{id}",
                            get(get_entity).put(update_entity).delete(delete_entity),
                        )
                        // UI Routes
                        .route("/api/ui/portal", get(get_portal))
                        .route("/api/ui/page/{entity}", get(get_page_config))
                        .route("/api/ui/form/{entity}", get(get_form_config))
                        .layer(
                            CorsLayer::new()
                                .allow_origin(Any)
                                .allow_methods(Any)
                                .allow_headers(Any),
                        )
                        .with_state(state);

                    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                        .await
                        .unwrap();
                    println!("🚀 Server running on http://0.0.0.0:{}", port);

                    // Run server with graceful shutdown
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async move {
                            tokio::signal::ctrl_c()
                                .await
                                .expect("failed to install CTRL+C handler");
                            println!("\n🛑 Shutdown signal received. Cleaning up...");

                            if let Some(mut child) = frontend_child {
                                println!("Killing frontend process...");
                                let _ = child.kill().await;
                            }
                        })
                        .await
                        .unwrap();
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
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn list_entities(
    State(state): State<AppState>,
    Path(entity): Path<String>,
) -> impl IntoResponse {
    match state.data_engine.list(&entity).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn get_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.data_engine.read(&entity, &id).await {
        Ok(Some(item)) => (StatusCode::OK, Json(item)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn update_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let ctx = RuntimeContext::system();
    match state.data_engine.update(&entity, &id, payload, &ctx).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "updated" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn delete_entity(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.data_engine.delete(&entity, &id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "deleted" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

// UI Handlers

async fn get_portal(State(state): State<AppState>) -> impl IntoResponse {
    let engine = PortalEngine::new();
    match engine.generate_navigation(state.data_engine.get_schema()) {
        Ok(nav) => (StatusCode::OK, Json(nav)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn get_page_config(
    State(state): State<AppState>,
    Path(entity): Path<String>,
) -> impl IntoResponse {
    let engine = PageEngine::new();
    match engine.generate_page_config(state.data_engine.get_schema(), &entity) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn get_form_config(
    State(state): State<AppState>,
    Path(entity): Path<String>,
) -> impl IntoResponse {
    let engine = FormEngine::new();
    // Default form name is 'default' for now, or use entity name as form name if we auto-generated it?
    // In current runtime/form.rs: `generate_ui_schema` takes `form_name`.
    // Usually forms are named in DSL. If we don't have explicit forms, we might fail.
    // However, the prompt implies "basic HR ERP".
    // Let's assume we want to generate a DEFAULT form for the entity if one doesn't exist?
    // Current FormEngine requires the form to exist in `schema.forms`.
    // Let's modify this to use "default" or maybe we need to update FormEngine to auto-generate from entity if missing.
    // For now, let's try to lookup a form named "{entity}_form" or just "{entity}".
    // DSL usually defined forms inside modules? KDL structure:
    // module "HR" { entity "Employee" ... form "employee_form" ... }
    // The current KDL DOES NOT HAVE FORMS defined.
    // SO FormEngine will fail.
    // I NEED TO UPDATE FormEngine to support auto-generation from Entity if no form is found, or Update KDL to include Basic Forms.
    // Updating FormEngine is robust.

    // Let's assume for this step I just call it with entity name and handle error.
    // Better: I will Update FormEngine in next steps if needed. For now, let's just Try using entity name.

    match engine.generate_ui_schema(state.data_engine.get_schema(), &entity) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(_) => match engine.generate_default_form(state.data_engine.get_schema(), &entity) {
            Ok(config) => (StatusCode::OK, Json(config)).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response(),
        },
    }
}
