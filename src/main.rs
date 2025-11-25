mod app_state;
mod note;
mod sql;        // now a folder with models, helpers, routes, crypto
mod not_found;  // 404 page module

use actix_files::Files;
use actix_web::{
    get,
    web::Data,
    App, HttpResponse, HttpServer, Responder,
};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use app_state::AppState;
use note::{note_get, note_post};
use not_found::go; // catch‑all route

static SHORTCUTS_FILE: &str = "shortcuts.json";
static NOTES_FILE: &str = "notes.json";

fn load_shortcuts() -> std::io::Result<HashMap<String, String>> {
    let data = fs::read_to_string(SHORTCUTS_FILE)?;
    let map: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

#[get("/")]
async fn index(state: Data<Arc<AppState>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&state.shortcuts).unwrap_or_else(|_| "{}".into()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load shortcuts
    let shortcuts = load_shortcuts().unwrap_or_else(|e| {
        eprintln!("Failed to load {SHORTCUTS_FILE}: {e}");
        HashMap::new()
    });

    // Load notes
    let notes_vec = if Path::new(NOTES_FILE).exists() {
        let data = fs::read_to_string(NOTES_FILE).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    // Shared application state
    let state = Arc::new(AppState {
        shortcuts,
        notes: Mutex::new(notes_vec),
        connections: Mutex::new(Vec::new()),
        last_results: Mutex::new(Vec::new()),
    });

    // Build server
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(index)
            .service(note_get)
            .service(note_post)
            .service(sql::sql_get)
            .service(sql::sql_add)
            .service(sql::sql_run)
            .service(sql::sql_export)
            .service(sql::sql_view)
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .service(go) // catch‑all route
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
