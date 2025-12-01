use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::Deserialize;
use std::{fs, io::{self, Write}, sync::Arc};
use serde_json;

use crate::app_state::{AppState, Theme, Note}; // MODIFIED: Import Note struct
// Import the shared HTML wrapper function from the new module
use crate::base_page::render_base_page;

static NOTES_FILE: &str = "notes.json";

#[derive(Deserialize)]
pub struct NoteForm {
    pub subject: String, // ADDED: Subject line
    pub content: String,
}

// FIX: Renamed to _DeleteForm to silence 'dead_code' warning
#[derive(Deserialize)]
pub struct _DeleteForm {
    pub note_index: usize, // The index of the note to delete
}

// MODIFIED: Accepts Vec<Note>
pub fn save_notes(notes: &[Note]) -> io::Result<()> {
    let json = serde_json::to_string(notes)?;
    let mut f = fs::File::create(NOTES_FILE)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

#[get("/note")]
pub async fn note_get(state: Data<Arc<AppState>>) -> impl Responder {
    // MODIFIED: notes is Vec<Note> now
    let notes = state.notes.lock().unwrap().clone();
    let current_theme = state.current_theme.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_note_page(&notes, &current_theme))
}

#[post("/note")]
pub async fn note_post(
    state: Data<Arc<AppState>>,
    form: web::Form<NoteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    
    let subject = form.subject.trim();
    let content = form.content.trim();
    
    // Check if either subject or content is non-empty
    if subject.is_empty() && content.is_empty() {
        // If empty, just redirect back
        return HttpResponse::SeeOther()
            .append_header(("Location", "/note"))
            .finish();
    }
    
    // 1. Determine the subject (auto-generate if missing)
    let final_subject = if subject.is_empty() {
        // Use the first 30 characters of content as the subject
        content.chars().take(30).collect::<String>().trim().to_string()
    } else {
        subject.to_string()
    };

    let new_note = Note {
        subject: final_subject.clone(),
        content: content.to_string(),
    };

    // 2. Overwrite Check / Update Logic: Check for existing note and perform update/overwrite
    // This handles the user request for an "overwrite checker" by treating a subject match as an update.
    let existing_index = notes.iter().position(|n| n.subject == final_subject);

    match existing_index {
        Some(index) => {
            // Overwrite: remove old note, insert new one (effectively updating in place)
            notes.remove(index);
            notes.insert(index, new_note);
            println!("Note updated: {}", final_subject);
        }
        None => {
            // New note: push to the end
            notes.push(new_note);
            println!("New note saved: {}", final_subject);
        }
    }
    
    save_notes(&notes).ok();
    
    // Redirect back to the note page. This ensures the form data is cleared.
    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}

// NEW HANDLER: POST /note/delete
pub async fn note_delete(
    state: Data<Arc<AppState>>,
    // FIX: Changed usage to _DeleteForm
    form: web::Form<_DeleteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    let index = form.note_index;

    if index < notes.len() {
        notes.remove(index);
        save_notes(&notes).ok();
        println!("Note deleted at index: {}", index);
    } else {
        eprintln!("Attempted to delete note with out-of-bounds index: {}", index);
    }

    // Redirect back to the note page
    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}


// MODIFIED: Function accepts Vec<Note>
fn render_note_page(notes: &[Note], current_theme: &Theme) -> String {
    // Render saved notes as clickable list items
    let rendered_notes = notes
        .iter()
        .enumerate() // ADDED: Enumerate to get the index for delete
        .map(|(index, n)| {
            // Store content, subject, and INDEX in data attributes.
            format!(
                r#"
                <li class="saved-note-item">
                    <span class="saved-note" data-index="{index}" data-subject="{subject}" data-content="{content}">
                        {subject_escaped}
                    </span>
                    <form method="POST" action="/note/delete" class="delete-form">
                        <input type="hidden" name="note_index" value="{index}">
                        <button type="submit" class="delete-button" title="Delete this note">🗑️</button>
                    </form>
                </li>
                "#,
                index = index,
                // Escape for use inside HTML attribute (data-content, data-subject)
                subject = encode_minimal(&n.subject),
                content = encode_minimal(&n.content),
                // Use minimal escape for display subject
                subject_escaped = encode_minimal(&n.subject),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // JavaScript for the textarea editor, including the new load feature
    let js = r#"
<script>
    const subjectInput = document.getElementById("subject");
    const textarea = document.getElementById("editor");
    const lineNumbers = document.getElementById("line-numbers");
    const savedNotesList = document.getElementById("saved-notes-list");
    const saveButton = document.querySelector('button[type="submit"]');

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
                    // Attempt to parse jsonish strings like {key:'val'}
                    let jsonish = val.replace(/'/g, '"');
                    let obj = JSON.parse(jsonish);
                    textarea.value = JSON.stringify(obj, null, 2);
                }
            } catch (err) {}
            updateLineNumbers();
        }, 0);
    });

    // NEW FEATURE: Load saved note into editor
    savedNotesList.addEventListener('click', (event) => {
        // Find the clickable subject span, ignoring the delete button/form
        const span = event.target.closest('.saved-note');
        if (span) {
            // The browser automatically decodes the HTML-escaped data attributes
            const subject = span.getAttribute('data-subject');
            const content = span.getAttribute('data-content');
            
            // 2. Populate the form fields (no confirmation, just overwrite)
            subjectInput.value = subject;
            textarea.value = content;
            
            // 3. Update line numbers for the new content
            updateLineNumbers();
            
            // 4. Update the save button text to indicate update action
            saveButton.textContent = "Update Note: " + subject;
            
            // 5. Give focus to the subject input for immediate editing
            subjectInput.focus();
        }
    });
    
    // Clear save button text when editing starts
    subjectInput.addEventListener('input', () => {
        if (saveButton.textContent.startsWith("Update Note:")) {
            saveButton.textContent = "Save or Update Note";
        }
    });

    textarea.addEventListener('input', () => {
        if (saveButton.textContent.startsWith("Update Note:")) {
            saveButton.textContent = "Save or Update Note";
        }
    });
    
    // The delete button submits a form directly to the backend.
    // NOTE: Confirmation modals (like window.confirm) are disabled here as per instructions.

    updateLineNumbers();
</script>
"#;

    // Add necessary CSS to style the subject input and list items, and the delete button
    let style = format!(
        r#"
<style>
    .subject-input {{
        width: 100%;
        padding: 10px;
        margin-bottom: 15px;
        border: 1px solid var(--border-color);
        background-color: var(--secondary-bg);
        color: var(--text-color);
        box-sizing: border-box;
        font-size: 1.1em;
        border-radius: 4px;
    }}
    .editor-container {{
        display: flex;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        overflow: hidden;
        margin-bottom: 15px;
    }}
    .line-numbers {{
        background-color: var(--tertiary-bg);
        color: #777;
        padding: 10px 5px;
        text-align: right;
        font-size: 0.9em;
        user-select: none;
        overflow: hidden;
        border-right: 1px solid var(--border-color);
        flex-shrink: 0;
        min-width: 30px;
    }}
    #editor {{
        flex-grow: 1;
        border: none;
        outline: none;
        padding: 10px;
        font-family: monospace;
        font-size: 1em;
        line-height: 1.2;
        resize: none;
        background-color: var(--secondary-bg);
        color: var(--text-color);
        height: 300px; /* Fixed height for editor */
    }}
    
    /* Styling for saved note list items */
    .saved-note-item {{
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 5px;
        background-color: var(--tertiary-bg);
        border-radius: 4px;
        padding: 0 0 0 12px;
    }}

    .saved-note {{
        cursor: pointer;
        padding: 8px 0;
        font-weight: bold;
        transition: color 0.2s;
        flex-grow: 1;
        /* Ensure the click area is the full subject span */
        display: block; 
    }}

    .saved-note:hover {{
        color: var(--link-hover);
    }}
    
    .delete-form {{
        margin: 0;
        line-height: 1;
        flex-shrink: 0;
    }}
    
    .delete-button {{
        background: none;
        border: none;
        cursor: pointer;
        color: var(--text-color);
        padding: 8px 12px;
        margin-left: 10px;
        font-size: 1em;
        transition: color 0.2s, background-color 0.2s;
        border-top-right-radius: 4px;
        border-bottom-right-radius: 4px;
        line-height: 1;
    }}
    .delete-button:hover {{
        background-color: #e00000; /* Red background on hover for delete */
        color: white;
    }}
    .saved-note-item::marker {{
        content: ""; /* Remove default list markers */
    }}
</style>
        "#,
    );

    // This is the custom content for the Notes page body.
    let content = format!(
        r#"
    <h1>Quick Notes</h1>
    <form method="POST" action="/note">
        <input type="text" id="subject" name="subject" placeholder="Subject Line (optional)" value="" class="subject-input" />
        <div class="editor-container">
            <div class="line-numbers" id="line-numbers"></div>
            <textarea id="editor" name="content"></textarea>
        </div>
        <button type="submit">Save or Update Note</button>
    </form>
    <h2>Saved Notes</h2>
    <ul id="saved-notes-list">
    {rendered_notes}
    </ul>
    {js}
    "#,
        rendered_notes = rendered_notes,
        js = js
    );

    // Use the reusable function to wrap the content with the base HTML and Nav Bar
    // Prepend the custom style to the content
    render_base_page("Quick Notes", &format!("{}{}", style, content), current_theme)
}