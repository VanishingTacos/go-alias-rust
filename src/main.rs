mod app_state;
mod note;
mod sql;        // now a folder with models, helpers, routes, crypto
mod not_found;  // 404 page module
mod base_page;  // New centralized module for base page helpers

use actix_files::Files;
use actix_web::{
    get, post,
    web::{Data, Form}, 
    App, HttpResponse, HttpServer, Responder,
};
use serde::Deserialize; 
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use app_state::AppState;
use note::{note_get, note_post};
// Import render_base_page from the new base_page module
use base_page::{render_base_page, render_add_shortcut_button, render_add_shortcut_modal, nav_bar_html};
// Import other necessary rendering helpers from not_found
use not_found::{go, render_shortcuts_table}; 

static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static NOTES_FILE: &str = "notes.json";

fn load_shortcuts(path: &str) -> std::io::Result<HashMap<String, String>> {
    let data = fs::read_to_string(path)?;
    let map: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

// ** NEW: Helper function to save shortcuts back to JSON file **
fn save_shortcuts(path: &str, shortcuts: &HashMap<String, String>) -> std::io::Result<()> {
    // Use serde_json::to_string_pretty for readable JSON
    let data = serde_json::to_string_pretty(shortcuts)?;
    fs::write(path, data)
}

// ** NEW: Struct to capture the form data **
#[derive(Deserialize)]
struct AddShortcutForm {
    shortcut: String,
    url: String,
    hidden: Option<String>, // HTML checkbox sends "on" or "true" if checked, nothing if not
}

// ** NEW: Handler for the new shortcut form **
#[post("/add_shortcut")]
async fn add_shortcut(
    form: Form<AddShortcutForm>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    let is_hidden = form.hidden.is_some();
    let shortcut = form.shortcut.trim();
    let url = form.url.trim();

    // Basic validation
    if shortcut.is_empty() || url.is_empty() {
        return HttpResponse::BadRequest().body("Shortcut and URL cannot be empty.");
    }

    if is_hidden {
        // Add to hidden shortcuts
        let mut hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
        hidden_shortcuts.insert(shortcut.to_string(), url.to_string());
        
        // Persist to disk
        if let Err(e) = save_shortcuts(HIDDEN_SHORTCUTS_FILE, &hidden_shortcuts) {
            eprintln!("Failed to save hidden shortcuts: {}", e);
            return HttpResponse::InternalServerError().body("Failed to save hidden shortcut.");
        }
    } else {
        // Add to visible shortcuts
        let mut shortcuts = state.shortcuts.lock().unwrap();
        shortcuts.insert(shortcut.to_string(), url.to_string());

        // Persist to disk
        if let Err(e) = save_shortcuts(SHORTCUTS_FILE, &shortcuts) {
            eprintln!("Failed to save shortcuts: {}", e);
            return HttpResponse::InternalServerError().body("Failed to save shortcut.");
        }
    }

    // Redirect back to the home page
    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}


#[get("/")]
async fn index(state: Data<Arc<AppState>>) -> impl Responder {
    let shortcuts = state.shortcuts.lock().unwrap();
    let table_html = render_shortcuts_table(&shortcuts);
    
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
    let html_output = render_base_page("Home - Shortcuts List", &full_page_content);
    
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
    
    let shortcuts = load_shortcuts(SHORTCUTS_FILE).unwrap_or_else(|e| {
        // ** CORRECTED: Removed extra 'T' **
        eprintln!("Failed to load {SHORTCUTS_FILE}: {e}"); 
        HashMap::new()
    });

    let hidden_shortcuts = load_shortcuts(HIDDEN_SHORTCUTS_FILE).unwrap_or_else(|e| {
        // ** CORRECTED: Removed extra 'T' **
        eprintln!("Failed to load {HIDDEN_SHORTCUTS_FILE}: {e}");
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
        // ** MODIFIED: Wrap in Mutex **
        shortcuts: Mutex::new(shortcuts),
        hidden_shortcuts: Mutex::new(hidden_shortcuts),
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
            .service(add_shortcut) 
            .service(go) // catch‑all route
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}