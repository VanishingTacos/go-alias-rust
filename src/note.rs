use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::Deserialize;
use std::{fs, io::{self, Write}, sync::Arc};
use serde_json;

use crate::app_state::AppState;
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
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_note_page(&notes))
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
    // Render the updated page
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_note_page(&notes))
}

fn render_note_page(notes: &[String]) -> String {
    let rendered_notes = notes
        .iter()
        .map(|n| format!("<li>{}</li>", encode_minimal(n)))
        .collect::<Vec<_>>()
        .join("\n");

    // JavaScript for the textarea editor
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
    render_base_page("Quick Notes", &content)
}