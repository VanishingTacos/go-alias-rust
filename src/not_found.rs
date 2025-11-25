use actix_web::{get, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;

/// Build grouped HTML table rows of shortcuts
fn grouped_shortcuts_table(shortcuts: &HashMap<String, String>) -> String {
    let mut grouped: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, url) in shortcuts.iter() {
        grouped.entry(url.as_str()).or_default().push(key.as_str());
    }

    let mut rows = String::new();
    let mut grouped_vec: Vec<_> = grouped.into_iter().collect();
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
     
/// Render the 404 page with available shortcuts
pub fn not_found_page(shortcuts: &HashMap<String, String>) -> String {
    let rows = grouped_shortcuts_table(shortcuts);
    format!(
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Shortcut Not Found</title>
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    <h1>404 – Shortcut Not Found</h1>

    <div class="tools">
      <h2>Tools</h2>
      <div class="tool-buttons">
        <a href="/sql"><button>SQL Manager</button></a>
        <a href="/note"><button>Notes</button></a>
      </div>
    </div>

    <p>Here are the available shortcuts:</p>
    <table class="grid">
      <thead>
        <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
  </body>
</html>"#,
        rows = rows
    )
}

/// Catch‑all route for shortcuts
#[get("/{path}")]
pub async fn go(path: web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    if let Some(url) = state.shortcuts.get(path.as_str()) {
        HttpResponse::Found()
            .append_header(("Location", url.clone()))
            .finish()
    } else {
        HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(not_found_page(&state.shortcuts))
    }
}
