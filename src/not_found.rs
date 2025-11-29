use actix_web::{get, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;
// Import render_base_page from the new centralized module
use crate::base_page::{render_base_page, render_add_shortcut_button, render_add_shortcut_modal, nav_bar_html};

/// Build grouped HTML table rows of shortcuts
fn grouped_shortcuts_table(shortcuts: &HashMap<String, String>) -> String {
    let mut grouped: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, url) in shortcuts.iter() {
        grouped.entry(url.as_str()).or_default().push(key.as_str());
    }

    let mut rows = String::new();
    let mut grouped_vec: Vec<_> = grouped.into_iter().collect();
    // Sort by URL first
    grouped_vec.sort_by_key(|(url, _)| url.to_owned());

    for (url, mut keys) in grouped_vec {
        keys.sort();
        let key_links = keys
            .iter()
            .map(|k| format!("<a href=\"/{0}\">{0}</a>", encode_minimal(k)))
            .collect::<Vec<_>>()
            .join(" , ");
        rows.push_str(&format!(
            "<tr><td class=\"keys\">{}</td><td class=\"url\">{}</td></tr>",
            key_links,
            encode_minimal(url)
        ));
    }
    rows
}

/// Renders the HTML table of shortcuts (reused by home and 404 pages).
pub fn render_shortcuts_table(shortcuts: &HashMap<String, String>) -> String {
    let rows = grouped_shortcuts_table(shortcuts);
    format!(
        r#"
    <table class="grid">
      <thead>
        <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
    "#,
        rows = rows
    )
}

/// Render the 404 page with available shortcuts
pub fn not_found_page(shortcuts: &HashMap<String, String>) -> String {
    let table = render_shortcuts_table(shortcuts);
    
    // We modify the navigation bar content for the 404 page only
    let nav_with_button = nav_bar_html()
        .replace(r#"<div id="optional-button-placeholder"></div>"#, &render_add_shortcut_button());

    let content = format!(
        r#"
    <h1>404 – Shortcut Not Found</h1>
    {}
    "#,
        table
    );
    
    // Use the imported render_base_page and replace the navigation content
    render_base_page("Shortcut Not Found", &content)
            // 1. Swap the navigation placeholder with the nav + button
            .replace(&nav_bar_html(), &nav_with_button)
            // 2. Append the modal just before </body>
            .replace("</body>", &format!("{}</body>", render_add_shortcut_modal()))
}

/// Catch‑all route for shortcuts
#[get("/{path}")]
pub async fn go(path: web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    // ** MODIFIED: Lock mutexes to read **
    let shortcuts = state.shortcuts.lock().unwrap();
    let hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();

    if let Some(url) = shortcuts.get(path.as_str())
        .or_else(|| hidden_shortcuts.get(path.as_str()))
    {
        HttpResponse::Found()
            .append_header(("Location", url.clone()))
            .finish()
    } else {
        HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(not_found_page(&shortcuts)) // only visible ones shown
    }
}