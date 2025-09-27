//! Static asset serving handlers
//!
//! This module handles serving static assets like HTML, CSS, and JavaScript
//! files for the admin UI.

use axum::{
    http::{self, StatusCode},
    response::{Html, IntoResponse},
};

/// Serve the main admin HTML page
pub async fn serve_admin_html() -> Html<&'static str> {
    Html(include_str!("../../ui/dist/index.html"))
}

/// Serve the admin CSS with proper content type
pub async fn serve_admin_css() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    (
        [(http::header::CONTENT_TYPE, "text/css")],
        include_str!("../../ui/dist/assets/index.css"),
    )
}

/// Serve the admin JavaScript with proper content type
pub async fn serve_admin_js() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    (
        [(http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../../ui/dist/assets/index.js"),
    )
}

/// Serve icon files
pub async fn serve_icon() -> impl IntoResponse {
    // Return a simple SVG icon or placeholder
    let icon_svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\"><rect width=\"32\" height=\"32\" fill=\"#4f46e5\"/><text x=\"16\" y=\"20\" text-anchor=\"middle\" fill=\"white\" font-family=\"Arial\" font-size=\"14\">MF</text></svg>";
    ([(http::header::CONTENT_TYPE, "image/svg+xml")], icon_svg)
}

/// Serve 32x32 icon
pub async fn serve_icon_32() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 48x48 icon
pub async fn serve_icon_48() -> impl IntoResponse {
    serve_icon().await
}

/// Serve logo files
pub async fn serve_logo() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 40x40 logo
pub async fn serve_logo_40() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 80x80 logo
pub async fn serve_logo_80() -> impl IntoResponse {
    serve_icon().await
}

/// Serve the API documentation as HTML
pub async fn serve_api_docs() -> Html<&'static str> {
    // Convert markdown to basic HTML for display
    let markdown = include_str!("../../API_DOCUMENTATION.md");
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MockForge Admin UI API Documentation</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f8f9fa;
        }}
        .container {{
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        h1, h2, h3 {{
            color: #2c3e50;
            margin-top: 30px;
        }}
        h1 {{ border-bottom: 2px solid #3498db; padding-bottom: 10px; }}
        h2 {{ border-bottom: 1px solid #bdc3c7; padding-bottom: 5px; }}
        code {{
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Monaco', 'Menlo', monospace;
        }}
        pre {{
            background: #f4f4f4;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
            border-left: 4px solid #3498db;
        }}
        .endpoint {{
            background: #e8f4fd;
            padding: 10px;
            margin: 10px 0;
            border-radius: 5px;
            border-left: 4px solid #3498db;
        }}
        .method {{
            font-weight: bold;
            color: #27ae60;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 8px;
            text-align: left;
        }}
        th {{
            background-color: #f2f2f2;
        }}
    </style>
</head>
<body>
    <div class="container">
        {}
    </div>
</body>
</html>"#,
        markdown_to_html(markdown)
    );
    Html(html.leak())
}

fn markdown_to_html(markdown: &str) -> String {
    // Very basic markdown to HTML converter
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();

    for line in markdown.lines() {
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</pre>\n");
                in_code_block = false;
                code_block_lang.clear();
            } else {
                in_code_block = true;
                code_block_lang = line.trim_start_matches('`').to_string();
                html.push_str("<pre><code>");
            }
        } else if in_code_block {
            html.push_str(&html_escape::encode_text(line));
            html.push('\n');
        } else if line.starts_with("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", &line[2..]));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", &line[3..]));
        } else if line.starts_with("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", &line[4..]));
        } else if line.starts_with("#### ") {
            html.push_str(&format!("<h4>{}</h4>\n", &line[5..]));
        } else if line.trim().is_empty() {
            html.push_str("<p></p>\n");
        } else if line.starts_with("- ") {
            html.push_str(&format!("<li>{}</li>\n", process_inline_markdown(&line[2..])));
        } else if line.starts_with("|") {
            // Basic table support
            if !html.ends_with("</table>\n") {
                html.push_str("<table>\n");
            }
            html.push_str("<tr>\n");
            for cell in line.split('|').skip(1) {
                if cell.trim().is_empty() {
                    continue;
                }
                html.push_str(&format!("<td>{}</td>\n", process_inline_markdown(cell.trim())));
            }
            html.push_str("</tr>\n");
        } else {
            html.push_str(&format!("<p>{}</p>\n", process_inline_markdown(line)));
        }
    }

    if in_code_block {
        html.push_str("</code></pre>\n");
    }

    html
}

fn process_inline_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Code spans
    result = result.replace("`", "<code>").replace("`", "</code>");

    // Bold
    result = result.replace("**", "<strong>").replace("**", "</strong>");

    // Italic
    result = result.replace("*", "<em>").replace("*", "</em>");

    // Links (basic)
    if let Some(start) = result.find('[') {
        if let Some(end) = result[start..].find(']') {
            if result[start + end + 1..].starts_with('(') {
                if let Some(link_end) = result[start + end + 2..].find(')') {
                    let link_text = &result[start + 1..start + end];
                    let link_url = &result[start + end + 2..start + end + 2 + link_end];
                    result = result.replace(
                        &result[start..start + end + 2 + link_end + 1],
                        &format!("<a href=\"{}\">{}</a>", link_url, link_text),
                    );
                }
            }
        }
    }

    result
}
