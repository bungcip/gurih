use axum::{
    Router,
    body::Body,
    extract::{Json, Multipart, Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    response::{Html, IntoResponse},
    routing::{MethodFilter, get, on, post},
};
use clap::{Parser, Subcommand};
use gurih_dsl::{compile, diagnostics::DiagnosticEngine, diagnostics::ErrorFormatter};
use gurih_ir::Symbol;
use gurih_runtime::action::ActionEngine;
use gurih_runtime::auth::AuthEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::form::FormEngine;
use gurih_runtime::page::PageEngine;
use gurih_runtime::portal::PortalEngine;
use notify::{RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

mod frontend;

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
        #[arg(long, env = "PORT", default_value = "3000")]
        port: u16,
        /// Only run the backend server, skip frontend and browser
        #[arg(long)]
        server_only: bool,
        /// Disable authentication (DANGEROUS: For dev/docs only)
        #[arg(long)]
        no_auth: bool,
    },
    /// Run the runtime in watch mode, restarting on file changes
    Watch {
        file: PathBuf,
        #[arg(long, env = "PORT", default_value = "3000")]
        port: u16,
        /// Only run the backend server, skip frontend and browser
        #[arg(long)]
        server_only: bool,
        /// Disable authentication
        #[arg(long)]
        no_auth: bool,
    },
    /// Generate fake test data for entities
    Faker {
        file: PathBuf,
        #[arg(long, default_value = "10")]
        count: Option<usize>,
    },
}

#[derive(Clone)]
struct AppState {
    data_engine: Arc<DataEngine>,
    action_engine: Arc<ActionEngine>,
    auth_engine: Arc<AuthEngine>,
    storage_engine: Arc<gurih_runtime::storage::StorageEngine>,
    no_auth: bool,
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
            no_auth,
        } => {
            if start_server(file, port, server_only, no_auth, false, true)
                .await
                .is_err()
            {
                std::process::exit(1);
            }
        }
        Commands::Watch {
            file,
            port,
            server_only,
            no_auth,
        } => {
            watch_loop(file, port, server_only, no_auth).await;
        }
        Commands::Faker { file, count } => match read_and_compile_with_diagnostics(&file) {
            Ok(schema) => {
                let schema = Arc::new(schema);
                let datastore = create_datastore(schema.clone(), &file).await;
                let faker = gurih_runtime::faker::FakerEngine::new();
                let count = count.unwrap_or(10);
                println!("🌱 Generating {} fake records per entity...", count);
                match faker.seed_entities(&schema, datastore.as_ref(), count).await {
                    Ok(_) => println!("✔ Faker completed successfully."),
                    Err(e) => {
                        eprintln!("❌ Faker failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(_) => std::process::exit(1),
        },
    }
}

#[derive(Debug)]
enum WatchEvent {
    Server,
    Frontend,
}

async fn watch_loop(file: PathBuf, port: u16, server_only: bool, no_auth: bool) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        match res {
            Ok(event) if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() => {
                // Check if KDL
                let is_kdl = event
                    .paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|ext| ext == "kdl"));

                // Check if Frontend (vue, js, css in web/src)
                let is_frontend = event.paths.iter().any(|p| {
                    let s = p.to_string_lossy();
                    (s.contains("/web/src") || s.contains("\\web\\src"))
                        && (p
                            .extension()
                            .is_some_and(|ext| ext == "vue" || ext == "js" || ext == "css" || ext == "html"))
                });

                if is_kdl {
                    let _ = tx.blocking_send(WatchEvent::Server);
                } else if is_frontend {
                    let _ = tx.blocking_send(WatchEvent::Frontend);
                }
            }
            _ => {}
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
            let _ = start_server(file_clone, port, server_only, no_auth, true, first_run).await;
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
                                frontend::rebuild_frontend();
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
    no_auth: bool,
    watch_mode: bool,
    open_browser: bool,
) -> Result<(), ()> {
    match read_and_compile_with_diagnostics(&file) {
        Ok(schema) => {
            let schema = Arc::new(schema);
            let datastore = create_datastore(schema.clone(), &file).await;

            let engine = Arc::new(DataEngine::new(schema.clone(), datastore.clone()));
            let auth_engine = Arc::new(AuthEngine::new(datastore));

            println!("Runtime initialized with {} entities.", schema.entities.len());

            let mut static_service = None;
            let mut web_dist_path = PathBuf::from("web/dist");

            if !server_only {
                if let Some(dist_dir) = frontend::ensure_frontend_built() {
                    static_service = Some(ServeDir::new(&dist_dir));
                    web_dist_path = dist_dir;
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

            // Initialize Storage Engine (File Storage)
            let storage_engine = Arc::new(gurih_runtime::storage::StorageEngine::new(&schema.storages).await);

            let mut app = Router::new()
                .route("/api/auth/login", post(login_handler))
                .route("/api/{entity}", post(create_entity).get(list_entities))
                .route(
                    "/api/{entity}/{id}",
                    get(get_entity).put(update_entity).delete(delete_entity),
                )
                .route("/api/upload/{entity}/{field}", post(upload_handler))
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
                auth_engine,
                storage_engine,
                no_auth,
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
                Err(())
            } else {
                Err(())
            }
        }
    }
}

async fn create_datastore(schema: Arc<gurih_ir::Schema>, file: &std::path::Path) -> Arc<dyn DataStore> {
    if schema.database.is_some() {
        sqlx::any::install_default_drivers();
        println!("🔌 Connecting to database...");
    }

    gurih_runtime::datastore::init_datastore(schema.clone(), file.parent())
        .await
        .expect("Failed to initialize datastore")
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

// Helper: Authentication Check
async fn check_auth(
    headers: HeaderMap,
    state: &AppState,
) -> Result<RuntimeContext, (StatusCode, Json<serde_json::Value>)> {
    if state.no_auth {
        return Ok(RuntimeContext::system());
    }
    #[allow(clippy::collapsible_if)]
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Some(ctx) = state.auth_engine.verify_token(token) {
                    return Ok(ctx);
                }
            }
        }
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "Unauthorized" })),
    ))
}

// Handlers

#[derive(serde::Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

async fn login_handler(State(state): State<AppState>, Json(payload): Json<LoginPayload>) -> impl IntoResponse {
    match state.auth_engine.login(&payload.username, &payload.password).await {
        Ok(ctx) => (StatusCode::OK, Json(ctx)).into_response(),
        Err(e) => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn create_entity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entity): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let ctx = match check_auth(headers, &state).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
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
    headers: HeaderMap,
    Path(entity): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    if let Err(e) = check_auth(headers, &state).await {
        return e.into_response();
    }
    match state.data_engine.list(&entity, params.limit, params.offset).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn get_entity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(e) = check_auth(headers, &state).await {
        return e.into_response();
    }
    match state.data_engine.read(&entity, &id).await {
        Ok(Some(item)) => (StatusCode::OK, Json(item)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Not found" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn update_entity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity, id)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let ctx = match check_auth(headers, &state).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match state.data_engine.update(&entity, &id, payload, &ctx).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "updated" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn delete_entity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let ctx = match check_auth(headers, &state).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match state.data_engine.delete(&entity, &id, &ctx).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "deleted" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn upload_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((entity_name, field_name)): Path<(String, String)>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Err(e) = check_auth(headers, &state).await {
        return e.into_response();
    }

    let schema = state.data_engine.get_schema();
    let entity = match schema.entities.get(&entity_name.as_str().into()) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Entity not found"})),
            )
                .into_response();
        }
    };

    let field = match entity
        .fields
        .iter()
        .find(|f| f.name == Symbol::from(field_name.as_str()))
    {
        Some(f) => f,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Field not found"})),
            )
                .into_response();
        }
    };

    let storage_name_string = field
        .storage
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "default".to_string());
    let storage_name = storage_name_string.as_str();

    if let Some(field_part) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field_part.file_name().unwrap_or("upload.bin").to_string();
        let data = field_part.bytes().await.unwrap_or_default();

        let data_bytes = if let Some(resize_dim) = &field.resize {
            match gurih_runtime::image_processor::resize_image(&data, resize_dim) {
                Ok(d) => bytes::Bytes::from(d),
                Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))).into_response(),
            }
        } else {
            data
        };

        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let unique_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);

        match state
            .storage_engine
            .upload(storage_name, &unique_name, data_bytes)
            .await
        {
            Ok(url) => {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "url": url,
                        "filename": unique_name,
                        "original_name": file_name
                    })),
                )
                    .into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response();
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "No file uploaded"})),
    )
        .into_response()
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
    match engine.generate_page_config(state.data_engine.get_schema(), Symbol::new(entity)) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn get_form_config(State(state): State<AppState>, Path(entity): Path<String>) -> impl IntoResponse {
    let engine = FormEngine::new();
    let entity = Symbol::new(entity);
    match engine.generate_ui_schema(state.data_engine.get_schema(), entity) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(_) => match engine.generate_default_form(state.data_engine.get_schema(), entity) {
            Ok(config) => (StatusCode::OK, Json(config)).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
        },
    }
}

async fn get_dashboard_data(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let roles = if let Some(role_hdr) = headers.get("x-gurih-role") {
        vec![role_hdr.to_str().unwrap_or("").to_string()]
    } else {
        vec![]
    };

    let engine = gurih_runtime::dashboard::DashboardEngine::new();
    match engine
        .evaluate(
            state.data_engine.get_schema(),
            Symbol::new(name),
            state.data_engine.datastore(),
            &roles,
        )
        .await
    {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn handle_dynamic_action(
    state: AppState,
    headers: HeaderMap,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
    action_name: String,
) -> impl IntoResponse {
    let ctx = match check_auth(headers, &state).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    // Merge params and query. Params override query.
    let mut args = query;
    for (k, v) in params {
        args.insert(k, v);
    }

    match state
        .action_engine
        .execute(&action_name, args, &state.data_engine, &ctx)
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
                if let Some(stripped) = s.strip_prefix(':') {
                    format!("{{{}}}", stripped)
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
        if route_def.action != Symbol::from("") && schema.actions.contains_key(&route_def.action) {
            let action_name = route_def.action;
            let handler = move |State(state): State<AppState>,
                                headers: HeaderMap,
                                Path(params): Path<HashMap<String, String>>,
                                Query(query): Query<HashMap<String, String>>| {
                handle_dynamic_action(state, headers, params, query, action_name.to_string())
            };

            match route_def.verb {
                gurih_ir::RouteVerb::All => {
                    app = app.route(&path, axum::routing::any(handler));
                }
                _ => {
                    let verb_filter = match route_def.verb {
                        gurih_ir::RouteVerb::Get => MethodFilter::GET,
                        gurih_ir::RouteVerb::Post => MethodFilter::POST,
                        gurih_ir::RouteVerb::Put => MethodFilter::PUT,
                        gurih_ir::RouteVerb::Delete => MethodFilter::DELETE,
                        _ => MethodFilter::GET,
                    };
                    app = app.route(&path, on(verb_filter, handler));
                }
            }
        }

        // Recursively register children
        app = register_routes(app, &path, route_def.children.iter(), schema);
    }
    app
}
