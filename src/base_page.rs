// base_page.rs

/// Generates the HTML for the reusable navigation bar, linking to all top-level tools.
pub fn nav_bar_html() -> String {
    r#"
    <div class="tools">
      <div class="tool-buttons">
        <a href="/"><button class="nav-button">Home (Shortcuts)</button></a>
        <a href="/sql"><button class="nav-button">SQL Manager</button></a>
        <a href="/note"><button class="nav-button">Notes</button></a>
      </div>
      <div id="optional-button-placeholder"></div>
    </div>
    "#.to_string()
}

/// Renders the standard HTML wrapper for any page.
/// Includes the required <head>, stylesheet, and the navigation bar.
pub fn render_base_page(title: &str, body_content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>{}</title>
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    {}
    {}
  </body>
</html>"#,
        title,
        nav_bar_html(), // Inject the reusable navigation bar
        body_content
    )
}

// ** NEW: Function to render just the Add Shortcut button **
pub fn render_add_shortcut_button() -> String {
    r#"
    <div class="add-shortcut-container">
        <button class="nav-button add-button" id="addShortcutBtn">+ Add Shortcut</button>
    </div>
    "#.to_string()
}

// ** NEW: Function to render the floating <dialog> and its JS **
pub fn render_add_shortcut_modal() -> String {
    let modal_html = r#"
<dialog id="addShortcutModal">
  <div class="modal-content">
    <span class="close-btn" id="closeModalBtn">&times;</span>
    <h2>Add New Shortcut</h2>
    <form action="/add_shortcut" method="POST" class="modal-form">
      <label for="shortcut">Shortcut:</label>
      <input type="text" id="shortcut" name="shortcut" placeholder="e.g., gh" required>

      <label for="url">URL:</label>
      <input type="url" id="url" name="url" placeholder="e.g., https://github.com" required>

      <div style="margin-top: 15px;">
        <input type="checkbox" id="hidden" name="hidden" value="true">
        <label for="hidden" style="display: inline; font-weight: normal;">Hidden?</label>
      </div>

      <div class="form-actions">
        <button type="submit" class="form-submit-btn">Save Shortcut</button>
      </div>
    </form>
  </div>
</dialog>
"#;

    let modal_js = r#"
<script>
  document.addEventListener('DOMContentLoaded', (event) => {
    var modal = document.getElementById("addShortcutModal");
    var btn = document.getElementById("addShortcutBtn");
    var span = document.getElementById("closeModalBtn");

    if (btn && modal) {
      btn.onclick = function() {
        modal.showModal(); 
      }
    }

    if (span && modal) {
      span.onclick = function() {
        modal.close();
      }
    }

    // Close modal if user clicks on the backdrop
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target.nodeName === 'DIALOG') {
                const rect = e.target.getBoundingClientRect();
                if (e.clientY < rect.top || e.clientY > rect.bottom ||
                    e.clientX < rect.left || e.clientX > rect.right) {
                    modal.close();
                }
            }
        });
    }
  });
</script>
"#;

    format!("{}{}", modal_html, modal_js)
}