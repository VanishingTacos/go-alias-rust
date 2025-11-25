use std::collections::HashMap;
use crate::sql::DbConnection;

pub fn find_connection<'a>(nick: &str, conns: &'a [DbConnection]) -> Option<&'a DbConnection> {
    conns.iter().find(|c| c.nickname == nick)
}


pub fn html_escape(s: &str) -> String {
    htmlescape::encode_minimal(s)
}

pub fn render_table(results: &[HashMap<String, String>]) -> String {
    if results.is_empty() {
        return "<pre>No output yet.</pre>".to_string();
    }
    let mut headers: Vec<String> = results[0].keys().cloned().collect();
    headers.sort();

    let thead = format!(
        "<tr>{}</tr>",
        headers.iter()
            .map(|h| format!("<th>{}</th>", html_escape(h)))
            .collect::<Vec<_>>()
            .join("")
    );
    let tbody = results.iter().map(|row| {
        let tds = headers.iter()
            .map(|h| html_escape(row.get(h).map(|v| v.as_str()).unwrap_or("")))
            .map(|v| format!("<td>{}</td>", v))
            .collect::<Vec<_>>()
            .join("");
        format!("<tr>{}</tr>", tds)
    }).collect::<Vec<_>>().join("\n");

    format!(
        "<table class=\"grid\"><thead>{}</thead><tbody>{}</tbody></table>",
        thead, tbody
    )
}
