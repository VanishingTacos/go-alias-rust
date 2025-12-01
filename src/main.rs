mod app_state;
mod note;
mod sql;        // now a folder with models, helpers, routes, crypto
mod not_found;  // 404 page module
mod base_page;  // New centralized module for base page helpers
mod elements;   // Module for elements

use actix_files::Files;
use actix_web::{
    get, 
    web::Data, 
    App, HttpResponse, HttpServer, Responder,
};
use std::{
    collections::HashMap,
    path::Path,
    fs,
    sync::{Arc, Mutex},
};

use app_state::AppState;
use note::{note_get, note_post};
// FIX: Removed unused imports (load_themes, load_current_theme, default_dark_theme)
use elements::theme::{get_settings, save_theme};
use elements::shortcut::{add_shortcut, delete_shortcut}; // IMPORTED delete_shortcut
// Import rendering helpers
use base_page::{render_base_page, render_add_shortcut_button, render_add_shortcut_modal, nav_bar_html};
// Import other necessary rendering helpers from not_found
use not_found::{go, render_shortcuts_table}; 

static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static WORK_SHORTCUTS_FILE: &str = "work-shortcuts.json"; 
static NOTES_FILE: &str = "notes.json";

// Only shortcut loading remains here
fn load_shortcuts(path: &str) -> std::io::Result<HashMap<String, String>> {
    let data = fs::read_to_string(path)?;
    let map: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

#[get("/")]
async fn index(state: Data<Arc<AppState>>) -> impl Responder {
    let shortcuts = state.shortcuts.lock().unwrap();
    let work_shortcuts = state.work_shortcuts.lock().unwrap(); 
    let current_theme = state.current_theme.lock().unwrap(); // Get current theme

    // Combine all *visible* shortcuts for display on the home page
    let mut combined_shortcuts = shortcuts.clone();
    combined_shortcuts.extend(work_shortcuts.clone());

    let table_html = render_shortcuts_table(&combined_shortcuts);
    
    // 1. Create the CUSTOM navigation bar with the Add Shortcut button injected into its placeholder.
    let nav_with_button = nav_bar_html()
        .replace(r#"<div id="optional-button-placeholder"></div>"#, &render_add_shortcut_button());
    
    let content = format!(
        r#"
        <p>Type a shortcut key into the URL bar (e.g., <code>/gh</code>) to go directly to the destination.</p>
        {}
        "#,
        table_html
    );

    // The content itself does not include the modal
    let full_page_content = content;

    // 2. Render the base page (which includes the *basic* navigation bar content).
    let html_output = render_base_page("Home - Shortcuts List", &full_page_content, &current_theme);
    
    // 3. Perform the replacements on the final output:
    let final_html = html_output
        // A. Swap the basic navigation bar with the one that includes the button.
        .replace(&nav_bar_html(), &nav_with_button)
        // B. Append the modal just before </body>.
        .replace("</body>", &format!("{}</body>", render_add_shortcut_modal()));

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(final_html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    
    // --- Shortcut Loading ---
    let shortcuts = load_shortcuts(SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {SHORTCUTS_FILE}: {e}"); 
        HashMap::new()
    });

    let hidden_shortcuts = load_shortcuts(HIDDEN_SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {HIDDEN_SHORTCUTS_FILE}: {e}");
        HashMap::new()
    });

    let work_shortcuts = load_shortcuts(WORK_SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {WORK_SHORTCUTS_FILE}: {e}");
        HashMap::new()
    });

    // --- Notes Loading ---
    let notes_vec = if Path::new(NOTES_FILE).exists() {
        let data = fs::read_to_string(NOTES_FILE).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    // --- Theme Loading ---
    let saved_themes = elements::theme::load_themes("themes.json").unwrap_or_else(|e| {
        eprintln!("Failed to load themes.json: {e}. Creating default map.");
        let mut map = HashMap::new();
        let default = elements::theme::default_dark_theme();
        map.insert(default.name.clone(), default);
        map
    });

    let current_theme = elements::theme::load_current_theme("current_theme.json").unwrap_or_else(|e| {
        eprintln!("Failed to load current_theme.json: {e}. Using default theme.");
        saved_themes.get("Dark Default").cloned().unwrap_or_else(elements::theme::default_dark_theme)
    });


    // Shared application state
    let state = Arc::new(AppState {
        shortcuts: Mutex::new(shortcuts),
        hidden_shortcuts: Mutex::new(hidden_shortcuts),
        work_shortcuts: Mutex::new(work_shortcuts),
        notes: Mutex::new(notes_vec),

        // THEME STATE
        current_theme: Mutex::new(current_theme),
        saved_themes: Mutex::new(saved_themes),

        // SQL service state
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
            .service(add_shortcut)      // POST /add_shortcut
            .service(delete_shortcut)   // NEW: POST /delete_shortcut
            .service(get_settings)      // GET /settings
            .service(save_theme)        // POST /save_theme
            .service(go) // catch‑all route
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}