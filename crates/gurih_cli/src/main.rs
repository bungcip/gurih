use axum::{
    Router,
    body::Body,
    extract::{Json, Path, Query, State},
    http::{Request, StatusCode},
    response::{Html, IntoResponse},
    routing::{MethodFilter, get, on, post},
};
use clap::{Parser, Subcommand};
use gurih_dsl::{compile, diagnostics::DiagnosticEngine, diagnostics::ErrorFormatter};
use gurih_runtime::action::ActionEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::form::FormEngine;
use gurih_runtime::page::PageEngine;
use gurih_runtime::persistence::SchemaManager;
use gurih_runtime::portal::PortalEngine;
use gurih_runtime::storage::{DatabaseStorage, MemoryStorage, Storage}; // Changed
use gurih_runtime::store::DbPool;
use notify::{RecursiveMode, Watcher};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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
    /// Run the runtime in watch mode, restarting on file changes
    Watch {
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
            if let Err(_) = start_server(file, port, server_only, false, true).await {
                std::process::exit(1);
            }
        }
        Commands::Watch {
            file,
            port,
            server_only,
        } => {
            watch_loop(file, port, server_only).await;
        }
    }
}

#[derive(Debug)]
enum WatchEvent {
    Server,
    Frontend,
}

fn rebuild_frontend() {
    println!("📦 Rebuilding frontend...");
    #[cfg(windows)]
    let npm_cmd = "npm.cmd";
    #[cfg(not(windows))]
    let npm_cmd = "npm";

    let web_dir = std::path::Path::new("web");
    if !web_dir.exists() {
        return;
    }

    let status = std::process::Command::new(npm_cmd)
        .arg("run")
        .arg("build")
        .current_dir(web_dir)
        .status();

    match status {
        Ok(s) if s.success() => println!("✅ Frontend rebuilt."),
        _ => eprintln!("❌ Frontend build failed."),
    }
}

async fn watch_loop(file: PathBuf, port: u16, server_only: bool) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                // Check if KDL
                let is_kdl = event
                    .paths
                    .iter()
                    .any(|p| p.extension().map_or(false, |ext| ext == "kdl"));

                // Check if Frontend (vue, js, css in web/src)
                let is_frontend = event.paths.iter().any(|p| {
                    let s = p.to_string_lossy();
                    (s.contains("/web/src") || s.contains("\\web\\src"))
                        && (p.extension().map_or(false, |ext| {
                            ext == "vue" || ext == "js" || ext == "css" || ext == "html"
                        }))
                });

                if is_kdl {
                    let _ = tx.blocking_send(WatchEvent::Server);
                } else if is_frontend {
                    let _ = tx.blocking_send(WatchEvent::Frontend);
                }
            }
        }
    })
    .expect("Failed to create watcher");

    // Watch the directory of the file (Server)
    let watch_path = if file.is_dir() {
        file.clone()
    } else {
        file.parent().unwrap_or(&PathBuf::from(".")).to_path_buf()
    };

    watcher
        .watch(&watch_path, RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    println!("👀 Watching for changes in {}", watch_path.display());

    // Watch Frontend
    if !server_only {
        let web_src = PathBuf::from("web/src");
        if web_src.exists() {
            let _ = watcher.watch(&web_src, RecursiveMode::Recursive);
            println!("👀 Watching for changes in web/src");
        }
    }

    let mut first_run = true;

    loop {
        // Spawn server
        let file_clone = file.clone();
        let server_task = tokio::spawn(async move {
            // For watch mode, we suppress some errors inside start_server or handle them?
            // start_server prints errors.
            let _ = start_server(file_clone, port, server_only, true, first_run).await;
        });

        first_run = false;

        // Inner loop to handle events
        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Some(WatchEvent::Server) => {
                            println!("\n🔄 Schema changed. Restarting server...");
                            server_task.abort();
                            // Wait a bit to ensure port is freed?
                            tokio::time::sleep(Duration::from_millis(200)).await;
                            break; // Break inner loop -> Restart server
                        }
                        Some(WatchEvent::Frontend) => {
                            // Rebuild frontend without killing server
                            // blocking task to allow build to finish
                            let _ = tokio::task::spawn_blocking(|| {
                                rebuild_frontend();
                            }).await;
                        }
                        None => return,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n🛑 Shutdown signal received.");
                    server_task.abort();
                    return;
                }
            }
        }
    }
}

async fn start_server(
    file: PathBuf,
    port: u16,
    server_only: bool,
    watch_mode: bool,
    open_browser: bool,
) -> Result<(), ()> {
    match read_and_compile_with_diagnostics(&file) {
        Ok(schema) => {
            println!("✔ Schema loaded. Starting runtime...");
            let schema = Arc::new(schema);

            // Initialize Storage
            let storage: Arc<dyn Storage> = if let Some(db_config) = &schema.database {
                sqlx::any::install_default_drivers();
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

                let pool = if db_config.db_type == "sqlite" {
                    let path = url
                        .trim_start_matches("sqlite://")
                        .trim_start_matches("sqlite:")
                        .trim_start_matches("file:");
                    let mut url = url.clone();
                    if !url.starts_with("sqlite:") {
                        url = format!("sqlite://{}", path);
                    }

                    let p = SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect(&url)
                        .await
                        .expect("Failed to connect to SQLite DB");
                    DbPool::Sqlite(p)
                } else if db_config.db_type == "postgresql" || db_config.db_type == "postgres" {
                    let p = PgPoolOptions::new()
                        .max_connections(5)
                        .connect(&url)
                        .await
                        .expect("Failed to connect to Postgres DB");
                    DbPool::Postgres(p)
                } else {
                    panic!("Unsupported database type: {}", db_config.db_type);
                };

                let manager = SchemaManager::new(pool.clone(), schema.clone(), db_config.db_type.clone());
                manager.migrate().await.expect("Migration failed");

                Arc::new(DatabaseStorage::new(pool))
            } else {
                println!("⚠️ No database configured. Using in-memory storage.");
                Arc::new(MemoryStorage::new())
            };

            let engine = Arc::new(DataEngine::new(schema.clone(), storage));

            println!("Runtime initialized with {} entities.", schema.entities.len());

            let mut static_service = None;
            let mut web_dist_path = PathBuf::from("web/dist");

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
                        static_service = Some(ServeDir::new(&dist_dir));
                        web_dist_path = dist_dir;
                    } else {
                        eprintln!("⚠️ Frontend build not found. Dashboard will not be available.");
                    }
                }

                // Open Browser
                if static_service.is_some() && open_browser {
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

            // Initialize Action Engine
            let action_engine = Arc::new(ActionEngine::new(schema.actions.clone()));

            let mut app = Router::new()
                .route("/api/{entity}", post(create_entity).get(list_entities))
                .route(
                    "/api/{entity}/{id}",
                    get(get_entity).put(update_entity).delete(delete_entity),
                )
                // UI Routes
                .route("/api/ui/portal", get(get_portal))
                .route("/api/ui/page/{entity}", get(get_page_config))
                .route("/api/ui/form/{entity}", get(get_form_config))
                .route("/api/ui/dashboard/{name}", get(get_dashboard_data));

            // Register Dynamic Routes for Actions
            app = register_routes(app, "", schema.routes.values(), &schema);

            let state = AppState {
                data_engine: engine,
                action_engine,
            };

            let mut app = app
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
                .with_state(state);

            if let Some(service) = static_service {
                if watch_mode {
                    // Inject script middleware/handler
                    let dist_path = web_dist_path.clone();
                    let index_handler = move |_req: Request<Body>| async move {
                        let path = dist_path.join("index.html");
                        match fs::read_to_string(path) {
                            Ok(mut content) => {
                                let script = r#"
<script>
(function() {
    let lost = false;
    function check() {
        fetch('/api/ui/portal').then(r => {
            if (r.ok) {
                if (lost) location.reload();
                lost = false;
                setTimeout(check, 1000);
            } else {
                lost = true;
                setTimeout(check, 500);
            }
        }).catch(() => {
            lost = true;
            setTimeout(check, 500);
        });
    }
    setTimeout(check, 1000);
})();
</script>
</body>"#;
                                content = content.replace("</body>", script);
                                Html(content).into_response()
                            }
                            Err(_) => StatusCode::NOT_FOUND.into_response(),
                        }
                    };

                    // Route / and /index.html explicitly to injector
                    app = app.route("/", get(index_handler.clone()));
                    app = app.route("/index.html", get(index_handler));
                }

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

            // Run server
            if watch_mode {
                // In watch mode, we don't handle Ctrl+C here because the loop handles it.
                // We just run until dropped (abort).
                axum::serve(listener, app).await.unwrap();
            } else {
                // In normal run mode, we handle graceful shutdown
                axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        tokio::signal::ctrl_c().await.expect("failed to install CTRL+C handler");
                        println!("\n🛑 Shutdown signal received. Cleaning up...");
                    })
                    .await
                    .unwrap();
            }

            Ok(())
        }
        Err(_) => {
            if watch_mode {
                println!("Waiting for changes...");
                // Keep the task alive (completed with error) so loop can restart on next change.
                // But the loop is: spawn -> select(rx, ctrl_c).
                // If this returns, the task finishes.
                // We need to keep the process running.
                // So if we return error, we should return Ok(()) to the loop, but maybe sleep?
                // Actually, if compilation fails, we print error and return Err.
                // The spawned task finishes.
                // The loop is `select! { _ = rx.recv(), _ = ctrlc }`.
                // It does NOT select on `server_task`.
                // So if server_task finishes (due to error), the loop sits waiting for `rx` (file change).
                // This is EXACTLY what we want!
                Err(())
            } else {
                Err(())
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

    let base_path = path.parent();
    match compile(&content, base_path) {
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
    match engine.generate_ui_schema(state.data_engine.get_schema(), &entity) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(_) => match engine.generate_default_form(state.data_engine.get_schema(), &entity) {
            Ok(config) => (StatusCode::OK, Json(config)).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
        },
    }
}

async fn get_dashboard_data(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    let engine = gurih_runtime::dashboard::DashboardEngine::new();
    match engine
        .evaluate(state.data_engine.get_schema(), &name, state.data_engine.storage())
        .await
    {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
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

fn register_routes<'a>(
    mut app: Router<AppState>,
    parent_path: &str,
    routes: impl Iterator<Item = &'a gurih_ir::RouteSchema>,
    schema: &gurih_ir::Schema,
) -> Router<AppState> {
    for route_def in routes {
        let segment_raw = route_def.path.trim_start_matches('/');
        let segment: String = segment_raw
            .split('/')
            .map(|s| {
                if s.starts_with(':') {
                    format!("{{{}}}", &s[1..])
                } else {
                    s.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/");

        let path = if parent_path == "/" || parent_path.is_empty() {
            format!("/{}", segment)
        } else if parent_path.ends_with('/') {
            format!("{}{}", parent_path, segment)
        } else {
            format!("{}/{}", parent_path, segment)
        };

        // Normalize path (ensure no double slash, though logic above should handle it)
        let path = path.replace("//", "/");

        // Only register if it is an actionable route (Action exists)
        if !route_def.action.is_empty() && schema.actions.contains_key(&route_def.action) {
            let verb_filter = match route_def.verb.as_str() {
                "GET" => MethodFilter::GET,
                "POST" => MethodFilter::POST,
                "PUT" => MethodFilter::PUT,
                "DELETE" => MethodFilter::DELETE,
                _ => MethodFilter::GET,
            };

            let action_name = route_def.action.clone();
            let handler = move |State(state): State<AppState>,
                                Path(params): Path<HashMap<String, String>>,
                                Query(query): Query<HashMap<String, String>>| {
                handle_dynamic_action(state, params, query, action_name)
            };

            // println!("Registering route: {} {} -> {}", route_def.verb, path, route_def.action);
            app = app.route(&path, on(verb_filter, handler));
        }

        // Recursively register children
        app = register_routes(app, &path, route_def.children.iter(), schema);
    }
    app
}
