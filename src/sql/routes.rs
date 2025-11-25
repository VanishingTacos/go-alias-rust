use actix_web::{get, post, web::{self, Data, Form}, HttpResponse, Responder};
use std::{collections::HashMap, sync::Arc};
use crate::app_state::AppState;
use crate::sql::{
    DbConnection, SqlForm, AddConnForm,
    find_connection, render_table,
    encrypt_and_save, load_and_decrypt,
};


#[get("/sql")]
pub async fn sql_get(state: Data<Arc<AppState>>) -> impl Responder {
    {
        let mut conns = state.connections.lock().unwrap();
        if conns.is_empty() {
            *conns = load_and_decrypt();
        }
    }
    let conns = state.connections.lock().unwrap().clone();

    let conn_links = conns.iter()
        .map(|c| format!(
            "<li><a href=\"/sql/{nick}\">{nick} ({db}@{host})</a></li>",
            nick = htmlescape::encode_minimal(&c.nickname),
            db = htmlescape::encode_minimal(&c.db_name),
            host = htmlescape::encode_minimal(&c.host)
        ))
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(r#"
<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>SQL Connections</title>
<link rel="stylesheet" href="/static/style.css"></head>
<body>
<h1>SQL Connections</h1>
<form method="POST" action="/sql/add">
  <input name="nickname" placeholder="Nickname" required>
  <input name="host" placeholder="Host" required>
  <input name="db_name" placeholder="Database Name" required>
  <input name="user" placeholder="User" required>
  <input name="password" type="password" placeholder="Password" required>
  <button type="submit">Save Connection</button>
</form>
<h2>Saved Connections</h2>
<ul>{conn_links}</ul>
</body></html>
"#, conn_links = conn_links);

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

#[post("/sql/add")]
pub async fn sql_add(form: Form<AddConnForm>, state: Data<Arc<AppState>>) -> impl Responder {
    let new_conn = DbConnection {
        host: form.host.clone(),
        db_name: form.db_name.clone(),
        user: form.user.clone(),
        password: form.password.clone(),
        nickname: form.nickname.clone(),
    };
    {
        let mut conns = state.connections.lock().unwrap();
        if let Some(idx) = conns.iter().position(|c| c.db_name == new_conn.db_name) {
            conns[idx] = new_conn;
        } else {
            conns.push(new_conn);
        }
        if let Err(e) = encrypt_and_save(&conns) {
            eprintln!("Failed to save encrypted connections: {e}");
        }
    }
    HttpResponse::Found().append_header(("Location", "/sql")).finish()
}

#[post("/sql/run")]
pub async fn sql_run(form: Form<SqlForm>, state: Data<Arc<AppState>>) -> impl Responder {
    use sqlx::{Row, Column, postgres::PgPoolOptions};

    let conn_opt = {
        let conns = state.connections.lock().unwrap();
        find_connection(&form.connection, &conns).cloned()
    };

    if conn_opt.is_none() {
        return HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body("<div style=\"color:#f88;\">Connection not found</div>");
    }

    let conn = conn_opt.unwrap();
    let dsn = format!("postgres://{}:{}@{}/{}", conn.user, conn.password, conn.host, conn.db_name);
    let pool = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(format!("<div style=\"color:#f88;\">DB connect error: {}</div>", e));
        }
    };

    let rows = match sqlx::query(&form.sql).fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(format!("<div style=\"color:#f88;\">Query error: {}</div>", e));
        }
    };

    let mut results_vec: Vec<HashMap<String, String>> = Vec::new();
    for row in rows {
        let mut map = HashMap::new();
        for col in row.columns() {
            let name = col.name().to_string();
            let val: Result<String, _> = row.try_get::<String, _>(name.as_str());
            map.insert(name, val.unwrap_or_default());
        }
        results_vec.push(map);
    }

    {
        let mut last = state.last_results.lock().unwrap();
        *last = results_vec.clone();
    }

    let table = render_table(&results_vec);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(table) // just the table HTML fragment
}

#[get("/sql/export")]
pub async fn sql_export(state: Data<Arc<AppState>>) -> impl Responder {
    let results = state.last_results.lock().unwrap();
    let mut wtr = csv::Writer::from_writer(vec![]);

    if results.is_empty() {
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap_or_default();
        return HttpResponse::Ok()
            .content_type("text/csv")
            .append_header(("Content-Disposition", "attachment; filename=\"results.csv\""))
            .body(data);
    }

    let mut headers: Vec<String> = results[0].keys().cloned().collect();
    headers.sort();
    wtr.write_record(&headers).ok();

    for row in results.iter() {
        let record: Vec<String> = headers.iter()
            .map(|h| row.get(h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&record).ok();
    }

    let data = match wtr.into_inner() {
        Ok(buf) => String::from_utf8(buf).unwrap_or_default(),
        Err(_) => "".to_string(),
    };

    HttpResponse::Ok()
        .content_type("text/csv")
        .append_header(("Content-Disposition", "attachment; filename=\"results.csv\""))
        .body(data)
}


#[get("/sql/{nickname}")]
pub async fn sql_view(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    use sqlx::{Row, postgres::PgPoolOptions};

    let nickname = path.into_inner();
    let conn_opt = {
        let conns = state.connections.lock().unwrap();
        conns.iter().find(|c| c.nickname == nickname).cloned()
    };
    let conn = match conn_opt {
        Some(c) => c,
        None => return HttpResponse::BadRequest().body("Connection not found"),
    };

    let dsn = format!(
        "postgres://{}:{}@{}/{}",
        conn.user, conn.password, conn.host, conn.db_name
    );
    let pool = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB connect error: {e}")),
    };

    // Fetch table names
    let rows = match sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public'"
    )
    .fetch_all(&pool)
    .await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list tables: {e}"))
        }
    };

    let tables: Vec<String> = rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("table_name").ok())
        .collect();

    // Build table list
    let table_list = tables
        .iter()
        .map(|t| {
            let safe = htmlescape::encode_minimal(t);
            format!(
                "<li><a href=\"#\" onclick=\"document.getElementById('sql-editor').value='SELECT * FROM {} LIMIT 100;'\">{}</a></li>",
                safe, safe
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Raw HTML template with placeholders
    let mut html = r#"
<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>{nickname}</title>
<link rel="stylesheet" href="/static/style.css"></head>
<body>

<div style="display:flex; height:100vh; position:relative;">
  <!-- Sidebar -->
  <div id="sidebar"
       style="width:200px; background:#333; color:#eee; padding:10px; overflow-y:auto; transition: width 0.3s, padding 0.3s;">
    <h2 style="margin:0;">Tables</h2>
    <ul>{table_list}</ul>
  </div>

  <!-- Arrow toggle -->
  <span id="toggle-arrow"
        style="position:fixed; top:10px; left:200px; cursor:pointer; font-size:18px; user-select:none; background:#444; color:#eee; padding:4px; border-radius:4px; transition:left 0.3s;">
    &#x25C0;
  </span>

  <!-- Main content -->
  <div id="main" style="flex:1; display:flex; flex-direction:column; padding:10px;">
    <form id="sql-form" method="POST" action="/sql/run">
      <input type="hidden" name="connection" value="{nickname}">
      <div class="editor-container">
        <textarea id="sql-editor" name="sql" placeholder="SELECT * FROM ..."></textarea>
      </div>
      <button type="submit">Run</button>
    </form>
    <div class="output" id="output"></div>
    <form method="GET" action="/sql/export">
      <button type="submit">Save as CSV</button>
    </form>
  </div>
</div>

<script>
  // Sidebar toggle
  const toggleArrow = document.getElementById('toggle-arrow');
  const sidebar = document.getElementById('sidebar');
  let collapsed = false;

  toggleArrow.addEventListener('click', () => {
    if (!collapsed) {
      sidebar.style.width = '0px';
      sidebar.style.padding = '0';
      toggleArrow.innerHTML = '&#x25B6;'; // ►
      toggleArrow.style.left = '0px';
      collapsed = true;
    } else {
      sidebar.style.width = '200px';
      sidebar.style.padding = '10px';
      toggleArrow.innerHTML = '&#x25C0;'; // ◀
      toggleArrow.style.left = '200px';
      collapsed = false;
    }
  });

  // AJAX form submit
  const form = document.getElementById('sql-form');
  const output = document.getElementById('output');

  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const formData = new FormData(form);
    const resp = await fetch('/sql/run', { method: 'POST', body: formData });
    const html = await resp.text();
    output.innerHTML = html;
  });
</script>

</body>
</html>
"#.to_string();

    // Replace placeholders
    html = html.replace("{nickname}", &htmlescape::encode_minimal(&nickname));
    html = html.replace("{table_list}", &table_list);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
