use axum::{
    Router,
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{MethodFilter, get, on, post},
};
use clap::{Parser, Subcommand};
use gurih_dsl::{compile, diagnostics::DiagnosticEngine, diagnostics::ErrorFormatter};
use gurih_runtime::action::ActionEngine; // Added
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::persistence::SchemaManager;
use gurih_runtime::storage::{AnyStorage, MemoryStorage, Storage};
use gurih_runtime::{form::FormEngine, page::PageEngine, portal::PortalEngine};
use serde_json::Value;
use sqlx::any::AnyPoolOptions;
use std::collections::HashMap; // Added
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

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
    action_engine: Arc<ActionEngine>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => match read_and_compile_with_diagnostics(&file) {
            Ok(_) => println!("✔ Schema is valid."),
            Err(_) => std::process::exit(1),
        },
        Commands::Build { file, output } => match read_and_compile_with_diagnostics(&file) {
            Ok(schema) => {
                let json = serde_json::to_string_pretty(&schema).unwrap();
                fs::write(output, json).expect("Failed to write output file");
                println!("✔ Build successful.");
            }
            Err(_) => std::process::exit(1),
        },
        Commands::Run {
            file,
            port,
            server_only,
        } => {
            match read_and_compile_with_diagnostics(&file) {
                Ok(schema) => {
                    println!("✔ Schema loaded. Starting runtime...");
                    let schema = Arc::new(schema);

                    // Initialize Storage
                    let storage: Arc<dyn Storage> = if let Some(db_config) = &schema.database {
                        sqlx::any::install_default_drivers();
                        println!("🔌 Connecting to database...");
                        // Handle env:DATABASE_URL
                        let mut url = if db_config.url.starts_with("env:") {
                            std::env::var(&db_config.url[4..]).unwrap_or_else(|_| "".to_string())
                        } else {
                            db_config.url.clone()
                        };

                        if url.is_empty() {
                            panic!("Database URL is empty or env var not set.");
                        }

                        if db_config.db_type == "sqlite" {
                            let path_str = url
                                .trim_start_matches("sqlite://")
                                .trim_start_matches("sqlite:")
                                .trim_start_matches("file:");

                            let path = std::path::Path::new(path_str);
                            let resolved_path = if path.is_relative() {
                                // Resolve relative to .kdl file location
                                let kdl_dir = file
                                    .parent()
                                    .unwrap_or_else(|| std::path::Path::new("."));
                                kdl_dir.join(path)
                            } else {
                                path.to_path_buf()
                            };

                            // Normalize path for usage
                            let resolved_path_str = resolved_path.to_str().expect("Invalid path encoding");

                            if !resolved_path.exists() {
                                println!("📝 Creating SQLite database file: {}", resolved_path_str);
                                // Ensure parent dir exists
                                if let Some(parent) = resolved_path.parent() {
                                    fs::create_dir_all(parent).ok();
                                }
                                fs::File::create(&resolved_path).expect("Failed to create SQLite database file");
                            }

                            // Update URL to point to the resolved absolute/relative path
                            url = format!("sqlite://{}", resolved_path_str);
                        }

                        let pool = AnyPoolOptions::new()
                            .max_connections(5)
                            .connect(&url)
                            .await
                            .expect("Failed to connect to DB");

                        let manager = SchemaManager::new(pool.clone(), schema.clone(), db_config.db_type.clone());
                        manager.migrate().await.expect("Migration failed");

                        Arc::new(AnyStorage::new(pool))
                    } else {
                        println!("⚠️ No database configured. Using in-memory storage.");
                        Arc::new(MemoryStorage::new())
                    };

                    let engine = Arc::new(DataEngine::new(schema.clone(), storage));

                    println!("Runtime initialized with {} entities.", schema.entities.len());

                    let mut static_service = None;

                    if !server_only {
                        let web_dir = std::path::Path::new("web");
                        let dist_dir = web_dir.join("dist");

                        if web_dir.exists() {
                            if !dist_dir.exists() {
                                println!("📦 Frontend build not found in web/dist. Attempting to build...");
                                #[cfg(windows)]
                                let npm_cmd = "npm.cmd";
                                #[cfg(not(windows))]
                                let npm_cmd = "npm";

                                // Install dependencies if needed? Assuming they might be missing.
                                // Actually, better to just run build. If node_modules missing, user should install.
                                // But let's try 'npm install && npm run build' equivalent.
                                let install_status = std::process::Command::new(npm_cmd)
                                    .arg("install")
                                    .current_dir(web_dir)
                                    .status();

                                if let Ok(status) = install_status {
                                    if status.success() {
                                        let build_status = std::process::Command::new(npm_cmd)
                                            .arg("run")
                                            .arg("build")
                                            .current_dir(web_dir)
                                            .status();

                                        if let Ok(b_status) = build_status {
                                            if !b_status.success() {
                                                eprintln!("⚠️ Failed to build frontend.");
                                            }
                                        } else {
                                            eprintln!("⚠️ Failed to run npm run build.");
                                        }
                                    } else {
                                        eprintln!("⚠️ Failed to run npm install.");
                                    }
                                } else {
                                    eprintln!("⚠️ Failed to run npm.");
                                }
                            }

                            if dist_dir.exists() {
                                println!("🚀 Serving frontend from {}", dist_dir.display());
                                static_service = Some(ServeDir::new(dist_dir));
                            } else {
                                eprintln!("⚠️ Frontend build not found. Dashboard will not be available.");
                            }
                        }

                        // Open Browser
                        if static_service.is_some() {
                            tokio::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                println!("🌐 Opening dashboard at http://localhost:{}", port);
                                #[cfg(windows)]
                                let _ = std::process::Command::new("cmd")
                                    .args(["/C", "start", &format!("http://localhost:{}", port)])
                                    .spawn();
                                #[cfg(not(windows))]
                                let _ = std::process::Command::new("open")
                                    .arg(format!("http://localhost:{}", port))
                                    .spawn();
                            });
                        }
                    }

                    // ... inside Run command ...
                    // Initialize Action Engine
                    let action_engine = Arc::new(ActionEngine::new(schema.actions.clone()));

                    // Fixed argument order: schema first, storage second
                    // let engine = Arc::new(DataEngine::new(schema.clone(), storage)); // Removed redundant init

                    // ...

                    let mut app = Router::new()
                        .route("/api/{entity}", post(create_entity).get(list_entities))
                        .route(
                            "/api/{entity}/{id}",
                            get(get_entity).put(update_entity).delete(delete_entity),
                        )
                        // UI Routes
                        .route("/api/ui/portal", get(get_portal))
                        .route("/api/ui/page/{entity}", get(get_page_config))
                        .route("/api/ui/form/{entity}", get(get_form_config));

                    // Register Dynamic Routes for Actions
                    for route_def in schema.routes.values() {
                        let path = if route_def.path.starts_with('/') {
                            route_def.path.clone()
                        } else {
                            format!("/{}", route_def.path)
                        };

                        // Convert Gurih route parameters ":id" to Axum ":id" (they are same if using colon)
                        // If my DSL uses ":id", Axum supports it. "api/{entity}" acts like ":entity" in new axum?
                        // Actually default is ":key". My usage of `{entity}` above might be old or compatible.
                        // Wait, looking at line 191: `.route("/api/{entity}"...` -> Axum 0.6+ uses `{key}`? No, Axum uses `:key`.
                        // But I see `{entity}` in the existing code. Maybe I should stick to what's there?
                        // IMPORTANT: Existing code uses `{entity}`. Let's assume `{}` syntax is valid or my DSL converts/uses it.
                        // My parser returns raw string. If KDL has `/api/:id`, I get `/api/:id`.
                        // If existing uses `/api/{entity}`, maybe it relies on strict match or I misread?
                        // Axum `matchit` supports `{param}` syntax since 0.7?
                        // Let's assume `{param}` is valid or I should normalize.
                        // I'll stick to what the route_def provides, assuming the user writes compatible paths.

                        let verb_filter = match route_def.verb.as_str() {
                            "GET" => MethodFilter::GET,
                            "POST" => MethodFilter::POST,
                            "PUT" => MethodFilter::PUT,
                            "DELETE" => MethodFilter::DELETE,
                            _ => MethodFilter::GET,
                        };

                        let action_name = route_def.action.clone();
                        let handler =
                            move |State(state): State<AppState>,
                                  Path(params): Path<HashMap<String, String>>,
                                  Query(query): Query<HashMap<String, String>>| {
                                handle_dynamic_action(state, params, query, action_name.clone())
                            };

                        app = app.route(&path, on(verb_filter, handler));
                    }

                    let state = AppState {
                        data_engine: engine,
                        action_engine,
                    };

                    let mut app = app
                        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
                        .with_state(state);

                    if let Some(service) = static_service {
                        app = app.fallback_service(service);
                    }

                    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                        .await
                        .unwrap_or_else(|e| {
                            panic!(
                                "💥 Failed to bind to port {}: {}. Tip: Try 'fuser -k {}/tcp' to free it.",
                                port, e, port
                            );
                        });
                    println!("🚀 Server running on http://0.0.0.0:{}", port);

                    // Run server with graceful shutdown
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async move {
                            tokio::signal::ctrl_c().await.expect("failed to install CTRL+C handler");
                            println!("\n🛑 Shutdown signal received. Cleaning up...");
                        })
                        .await
                        .unwrap();
                }
                Err(_) => std::process::exit(1),
            }
        }
    }
}

fn read_and_compile_with_diagnostics(path: &PathBuf) -> Result<gurih_ir::Schema, ()> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            return Err(());
        }
    };

    match compile(&content) {
        Ok(schema) => Ok(schema),
        Err(e) => {
            let mut engine = DiagnosticEngine::new();
            engine.report(e);

            let formatter = ErrorFormatter::new();
            let filename = path.to_str().unwrap_or("<unknown>");

            for diag in engine.diagnostics() {
                eprintln!("{}", formatter.format_diagnostic(diag, &content, filename));
            }

            Err(())
        }
    }
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

#[derive(serde::Deserialize)]
struct ListParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn list_entities(
    State(state): State<AppState>,
    Path(entity): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match state.data_engine.list(&entity, params.limit, params.offset).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn get_entity(State(state): State<AppState>, Path((entity, id)): Path<(String, String)>) -> impl IntoResponse {
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

async fn delete_entity(State(state): State<AppState>, Path((entity, id)): Path<(String, String)>) -> impl IntoResponse {
    match state.data_engine.delete(&entity, &id).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "deleted" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
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

async fn get_page_config(State(state): State<AppState>, Path(entity): Path<String>) -> impl IntoResponse {
    let engine = PageEngine::new();
    match engine.generate_page_config(state.data_engine.get_schema(), &entity) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn get_form_config(State(state): State<AppState>, Path(entity): Path<String>) -> impl IntoResponse {
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
            Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
        },
    }
}

async fn handle_dynamic_action(
    state: AppState,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
    action_name: String,
) -> impl IntoResponse {
    // Merge params and query. Params override query.
    let mut args = query;
    for (k, v) in params {
        args.insert(k, v);
    }

    match state
        .action_engine
        .execute(&action_name, args, &state.data_engine)
        .await
    {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!({ "message": resp.message }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}
