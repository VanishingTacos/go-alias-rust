use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::Deserialize;
use std::{fs, io::{self, Write}, sync::Arc};
use serde_json;

use crate::app_state::{AppState, Theme}; // ADDED: Import Theme struct
// Import the shared HTML wrapper function from the new module
use crate::base_page::render_base_page;

static NOTES_FILE: &str = "notes.json";

#[derive(Deserialize)]
pub struct NoteForm {
    pub content: String,
}

pub fn save_notes(notes: &[String]) -> io::Result<()> {
    let json = serde_json::to_string(notes)?;
    let mut f = fs::File::create(NOTES_FILE)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

#[get("/note")]
pub async fn note_get(state: Data<Arc<AppState>>) -> impl Responder {
    let notes = state.notes.lock().unwrap().clone();
    // ADDED: Lock the current theme state
    let current_theme = state.current_theme.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        // MODIFIED: Pass the current_theme reference
        .body(render_note_page(&notes, &current_theme))
}

#[post("/note")]
pub async fn note_post(
    state: Data<Arc<AppState>>,
    form: web::Form<NoteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    if !form.content.trim().is_empty() {
        notes.push(form.content.clone());
        save_notes(&notes).ok();
    }
    
    // ADDED: Lock the theme for rendering
    let current_theme = state.current_theme.lock().unwrap();

    // Render the updated page
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        // MODIFIED: Pass the current_theme reference
        .body(render_note_page(&notes, &current_theme))
}

// MODIFIED: Function now accepts the current_theme argument
fn render_note_page(notes: &[String], current_theme: &Theme) -> String {
    let rendered_notes = notes
        .iter()
        .map(|n| format!("<li>{}</li>", encode_minimal(n)))
        .collect::<Vec<_>>()
        .join("\n");

    // JavaScript for the textarea editor
    // NOTE: Curly braces inside the JS string literal must be escaped {{ and }} 
    // when using format! macro, but since this is defined outside the final 
    // format! call, we only need to ensure they are escaped for the final 
    // render_base_page call. Since the JS is passed as a standalone string 
    // it will be correctly rendered in the final HTML string.
    let js = r#"
<script>
    const textarea = document.getElementById("editor");
    const lineNumbers = document.getElementById("line-numbers");

    function updateLineNumbers() {
        const lines = textarea.value.split("\n").length;
        lineNumbers.innerHTML = "";
        for (let i = 1; i <= lines; i++) {
            const div = document.createElement("div");
            div.textContent = i;
            lineNumbers.appendChild(div);
        }
    }

    textarea.addEventListener("scroll", () => {
        lineNumbers.scrollTop = textarea.scrollTop;
    });

    textarea.addEventListener("input", updateLineNumbers);

    textarea.addEventListener("paste", function() {
        setTimeout(() => {
            try {
                let val = textarea.value.trim();
                // Attempt basic JSON pretty printing on paste
                if (val.startsWith("{") || val.startsWith("[")) {
                    let obj = JSON.parse(val);
                    textarea.value = JSON.stringify(obj, null, 2);
                } else if (val.includes("{") && val.includes(":")) {
                    let jsonish = val.replace(/'/g, '"');
                    let obj = JSON.parse(jsonish);
                    textarea.value = JSON.stringify(obj, null, 2);
                }
            } catch (err) {}
            updateLineNumbers();
        }, 0);
    });

    updateLineNumbers();
</script>
"#;

    // This is the custom content for the Notes page body.
    let content = format!(
        r#"
    <h1>Quick Notes</h1>
    <form method="POST" action="/note">
        <div class="editor-container">
            <div class="line-numbers" id="line-numbers"></div>
            <textarea id="editor" name="content"></textarea>
        </div>
        <button type="submit">Save</button>
    </form>
    <h2>Saved Notes</h2>
    <ul>
    {rendered_notes}
    </ul>
    {js}
    "#,
        rendered_notes = rendered_notes,
        js = js
    );

    // Use the reusable function to wrap the content with the base HTML and Nav Bar
    // MODIFIED: Pass current_theme
    render_base_page("Quick Notes", &content, current_theme)
}