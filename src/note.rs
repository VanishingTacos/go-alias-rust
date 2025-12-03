use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::Deserialize;
use std::{fs, io::{self, Write}, sync::Arc};
use serde_json;

use crate::app_state::{AppState, Theme, Note};
use crate::base_page::render_base_page;

static NOTES_FILE: &str = "notes.json";

#[derive(Deserialize)]
pub struct NoteForm {
    pub subject: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct _DeleteForm {
    pub note_index: usize,
}

pub fn save_notes(notes: &[Note]) -> io::Result<()> {
    let json = serde_json::to_string(notes)?;
    let mut f = fs::File::create(NOTES_FILE)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

#[get("/note")]
pub async fn note_get(state: Data<Arc<AppState>>) -> impl Responder {
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
    
    if subject.is_empty() && content.is_empty() {
        return HttpResponse::SeeOther()
            .append_header(("Location", "/note"))
            .finish();
    }
    
    let final_subject = if subject.is_empty() {
        content.chars().take(30).collect::<String>().trim().to_string()
    } else {
        subject.to_string()
    };

    let new_note = Note {
        subject: final_subject.clone(),
        content: content.to_string(),
    };

    let existing_index = notes.iter().position(|n| n.subject == final_subject);

    match existing_index {
        Some(index) => {
            notes.remove(index);
            notes.insert(index, new_note);
            println!("Note updated: {}", final_subject);
        }
        None => {
            notes.push(new_note);
            println!("New note saved: {}", final_subject);
        }
    }
    
    save_notes(&notes).ok();
    
    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}

pub async fn note_delete(
    state: Data<Arc<AppState>>,
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

    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}


fn render_note_page(notes: &[Note], current_theme: &Theme) -> String {
    let rendered_notes = notes
        .iter()
        .enumerate()
        .map(|(index, n)| {
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
                subject = encode_minimal(&n.subject),
                content = encode_minimal(&n.content),
                subject_escaped = encode_minimal(&n.subject),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let js = r#"
<script>
    const subjectInput = document.getElementById("subject");
    const textarea = document.getElementById("editor");
    const lineNumbers = document.getElementById("line-numbers");
    const savedNotesList = document.getElementById("saved-notes-list");
    const saveButton = document.getElementById("save-btn");
    const fileInput = document.getElementById('file-input');
    const previewContainer = document.getElementById('markdown-preview');
    const editorContainer = document.querySelector('.editor-container');
    const togglePreviewBtn = document.getElementById('toggle-preview-btn');

    let isPreview = false;

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

    textarea.addEventListener("input", () => {
        updateLineNumbers();
        resetSaveButton();
    });

    // Re-added JSON Auto-formatting on paste
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
    
    subjectInput.addEventListener("input", resetSaveButton);

    function resetSaveButton() {
        if (saveButton.textContent.startsWith("Update Note:")) {
            saveButton.textContent = "Save or Update Note";
        }
    }

    // --- Basic Markdown Parser (Client-side) ---
    function renderMarkdown(text) {
        // Simple regex-based parser for basic MD support
        let html = text
            .replace(/^# (.*$)/gim, '<h1>$1</h1>')
            .replace(/^## (.*$)/gim, '<h2>$1</h2>')
            .replace(/^### (.*$)/gim, '<h3>$1</h3>')
            .replace(/^\> (.*$)/gim, '<blockquote>$1</blockquote>')
            .replace(/\*\*(.*)\*\*/gim, '<b>$1</b>')
            .replace(/\*(.*)\*/gim, '<i>$1</i>')
            .replace(/`([^`]+)`/gim, '<code>$1</code>')
            .replace(/```([^`]+)```/gim, '<pre><code>$1</code></pre>')
            .replace(/\[(.*?)\]\((.*?)\)/gim, "<a href='$2' target='_blank'>$1</a>")
            .replace(/\n/gim, '<br />');
        return html;
    }

    // --- Button Handlers ---

    // Toggle Markdown Preview
    togglePreviewBtn.addEventListener('click', (e) => {
        e.preventDefault(); // Prevent form submit
        isPreview = !isPreview;
        
        if (isPreview) {
            previewContainer.innerHTML = renderMarkdown(textarea.value);
            editorContainer.style.display = 'none';
            previewContainer.style.display = 'block';
            togglePreviewBtn.textContent = 'Edit Text';
        } else {
            editorContainer.style.display = 'flex';
            previewContainer.style.display = 'none';
            togglePreviewBtn.textContent = 'Preview Markdown';
        }
    });

    // Open Local File
    document.getElementById('open-file-btn').addEventListener('click', (e) => {
        e.preventDefault();
        fileInput.click();
    });

    fileInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (e) => {
            textarea.value = e.target.result;
            subjectInput.value = file.name; // Set subject as filename
            updateLineNumbers();
            resetSaveButton();
            
            // If currently previewing, update preview
            if(isPreview) {
                 previewContainer.innerHTML = renderMarkdown(textarea.value);
            }
        };
        reader.readAsText(file);
        // Reset input so same file can be selected again if needed
        fileInput.value = '';
    });

    // Save/Download as File
    document.getElementById('download-btn').addEventListener('click', (e) => {
        e.preventDefault();
        const text = textarea.value;
        if (!text) {
            alert("Note is empty!");
            return;
        }
        
        let name = subjectInput.value.trim();
        if (!name) name = 'note.txt';
        
        // Ensure extension if missing
        if (!name.includes('.')) {
            name += '.txt';
        }

        const blob = new Blob([text], { type: 'text/plain' });
        const anchor = document.createElement('a');
        anchor.download = name;
        anchor.href = window.URL.createObjectURL(blob);
        anchor.target = '_blank';
        anchor.style.display = 'none';
        document.body.appendChild(anchor);
        anchor.click();
        document.body.removeChild(anchor);
    });

    // Compose Email (Gmail)
    document.getElementById('email-btn').addEventListener('click', (e) => {
        e.preventDefault();
        
        const subject = subjectInput.value.trim();
        const body = textarea.value.trim();
        
        // Use encodeURIComponent to ensure special characters are handled in the URL
        const encodedSubject = encodeURIComponent(subject);
        const encodedBody = encodeURIComponent(body);
        
        // Gmail compose URL
        const gmailUrl = `https://mail.google.com/mail/?view=cm&fs=1&su=${encodedSubject}&body=${encodedBody}`;
        
        window.open(gmailUrl, '_blank');
    });

    // --- History Logic ---
    savedNotesList.addEventListener('click', (event) => {
        const span = event.target.closest('.saved-note');
        if (span) {
            const subject = span.getAttribute('data-subject');
            const content = span.getAttribute('data-content');
            
            subjectInput.value = subject;
            textarea.value = content;
            
            updateLineNumbers();
            saveButton.textContent = "Update Note: " + subject;
            
            // If previewing, update preview immediately
            if (isPreview) {
                previewContainer.innerHTML = renderMarkdown(content);
            } else {
                subjectInput.focus();
            }
        }
    });

    updateLineNumbers();
</script>
"#;

    let style = format!(
        r#"
<style>
    .toolbar {{
        display: flex;
        gap: 10px;
        margin-bottom: 10px;
        align-items: center;
    }}
    .subject-input {{
        flex-grow: 1;
        padding: 10px;
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
        min-height: 400px;
    }}
    /* Markdown Preview Styles */
    #markdown-preview {{
        display: none; /* Hidden by default */
        border: 1px solid var(--border-color);
        border-radius: 4px;
        padding: 20px;
        background-color: var(--secondary-bg);
        color: var(--text-color);
        min-height: 400px;
        overflow-y: auto;
        margin-bottom: 15px;
    }}
    #markdown-preview h1, #markdown-preview h2 {{ border-bottom: 1px solid var(--border-color); padding-bottom: 5px; }}
    #markdown-preview code {{ background: #444; padding: 2px 5px; border-radius: 3px; }}
    #markdown-preview pre {{ background: #333; padding: 10px; border-radius: 5px; overflow-x: auto; }}
    #markdown-preview blockquote {{ border-left: 3px solid var(--link-color); margin-left: 0; padding-left: 10px; color: #aaa; }}

    .line-numbers {{
        background-color: var(--tertiary-bg);
        color: #777;
        padding: 10px 5px;
        text-align: right;
        font-family: monospace; /* FIX: Ensure font matches editor */
        font-size: 1em;         /* FIX: Match editor font size exactly */
        line-height: 1.2;       /* FIX: Match editor line height */
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
        white-space: pre; /* FIX: Prevent wrapping so lines match numbers 1:1 */
        overflow-x: auto; /* FIX: Allow scrolling for long lines */
        resize: none;
        background-color: var(--secondary-bg);
        color: var(--text-color);
        height: 400px; 
    }}
    
    /* Saved Notes List */
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
        display: block; 
    }}
    .saved-note:hover {{ color: var(--link-hover); }}
    
    .delete-form {{ margin: 0; line-height: 1; flex-shrink: 0; }}
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
        margin-top: 0; /* Override default button margin */
    }}
    .delete-button:hover {{ background-color: #e00000; color: white; }}
    .saved-note-item::marker {{ content: ""; }}
    
    /* Utility Buttons */
    .utility-btn {{
        margin-top: 0; /* Reset default */
        margin-right: 5px;
        background-color: var(--tertiary-bg);
        border: 1px solid var(--border-color);
    }}
    .utility-btn:hover {{ background-color: var(--border-color); }}
</style>
        "#,
    );

    let content = format!(
        r#"
    <h1>Quick Notes & Markdown Editor</h1>
    <form method="POST" action="/note">
        <div class="toolbar">
            <!-- Hidden file input for opening files -->
            <input type="file" id="file-input" style="display: none;" accept=".txt,.md,.json,.rs,.js,.html">
            
            <button type="button" id="open-file-btn" class="utility-btn">📂 Open File</button>
            <input type="text" id="subject" name="subject" placeholder="Subject / Filename" value="" class="subject-input" />
            <button type="button" id="toggle-preview-btn" class="utility-btn">Preview Markdown</button>
            <button type="button" id="download-btn" class="utility-btn">💾 Save to Disk</button>
            <button type="button" id="email-btn" class="utility-btn">📧 Email</button>
        </div>

        <div class="editor-container">
            <div class="line-numbers" id="line-numbers"></div>
            <textarea id="editor" name="content" placeholder="Type here or drop a file..."></textarea>
        </div>
        
        <div id="markdown-preview"></div>

        <button type="submit" id="save-btn" style="width: 100%; padding: 10px; font-size: 1.1em;">Save or Update Note (Database)</button>
    </form>
    
    <h2>Saved Notes (Database)</h2>
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