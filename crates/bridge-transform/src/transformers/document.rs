//! Document format transformations (Markdown, HTML, etc.)

use crate::{Format, Result, TransformError, TransformOptions};
use pulldown_cmark::{html, Options, Parser};

/// Transform document between formats
pub fn transform_document(
    data: &[u8],
    from: Format,
    to: Format,
    _options: &TransformOptions,
) -> Result<Vec<u8>> {
    let input = std::str::from_utf8(data)
        .map_err(|e| TransformError::Failed(e.to_string()))?;

    let output = match (from, to) {
        (Format::Markdown, Format::Html) => markdown_to_html(input),
        (Format::Html, Format::Markdown) => html_to_markdown(input)?,
        (Format::Html, Format::PlainText) => html_to_plain_text(input),
        (Format::Markdown, Format::PlainText) => markdown_to_plain_text(input),
        _ => {
            return Err(TransformError::UnsupportedFormat(format!(
                "Cannot transform {:?} to {:?}",
                from, to
            )));
        }
    };

    Ok(output.into_bytes())
}

/// Convert Markdown to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

/// Convert Markdown to HTML with full document wrapper
pub fn markdown_to_html_document(markdown: &str, title: &str) -> String {
    let body = markdown_to_html(markdown);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            color: #333;
        }}
        pre {{
            background: #f4f4f4;
            padding: 1rem;
            border-radius: 4px;
            overflow-x: auto;
        }}
        code {{
            background: #f4f4f4;
            padding: 0.2rem 0.4rem;
            border-radius: 2px;
            font-size: 0.9em;
        }}
        pre code {{
            background: none;
            padding: 0;
        }}
        blockquote {{
            border-left: 4px solid #ddd;
            margin-left: 0;
            padding-left: 1rem;
            color: #666;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 0.5rem;
            text-align: left;
        }}
        th {{
            background: #f4f4f4;
        }}
        img {{
            max-width: 100%;
        }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
        title, body
    )
}

/// Convert HTML to Markdown (basic implementation)
pub fn html_to_markdown(html: &str) -> Result<String> {
    // This is a simplified implementation
    // A full implementation would use a proper HTML parser

    let mut markdown = html.to_string();

    // Headers
    markdown = regex::Regex::new(r"<h1[^>]*>(.*?)</h1>")
        .unwrap()
        .replace_all(&markdown, "# $1\n\n")
        .to_string();
    markdown = regex::Regex::new(r"<h2[^>]*>(.*?)</h2>")
        .unwrap()
        .replace_all(&markdown, "## $1\n\n")
        .to_string();
    markdown = regex::Regex::new(r"<h3[^>]*>(.*?)</h3>")
        .unwrap()
        .replace_all(&markdown, "### $1\n\n")
        .to_string();
    markdown = regex::Regex::new(r"<h4[^>]*>(.*?)</h4>")
        .unwrap()
        .replace_all(&markdown, "#### $1\n\n")
        .to_string();

    // Paragraphs
    markdown = regex::Regex::new(r"<p[^>]*>(.*?)</p>")
        .unwrap()
        .replace_all(&markdown, "$1\n\n")
        .to_string();

    // Bold
    markdown = regex::Regex::new(r"<(strong|b)[^>]*>(.*?)</\1>")
        .unwrap()
        .replace_all(&markdown, "**$2**")
        .to_string();

    // Italic
    markdown = regex::Regex::new(r"<(em|i)[^>]*>(.*?)</\1>")
        .unwrap()
        .replace_all(&markdown, "*$2*")
        .to_string();

    // Code
    markdown = regex::Regex::new(r"<code[^>]*>(.*?)</code>")
        .unwrap()
        .replace_all(&markdown, "`$1`")
        .to_string();

    // Links
    markdown = regex::Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#)
        .unwrap()
        .replace_all(&markdown, "[$2]($1)")
        .to_string();

    // Images
    markdown = regex::Regex::new(r#"<img[^>]*src="([^"]*)"[^>]*alt="([^"]*)"[^>]*/?\s*>"#)
        .unwrap()
        .replace_all(&markdown, "![$2]($1)")
        .to_string();

    // List items
    markdown = regex::Regex::new(r"<li[^>]*>(.*?)</li>")
        .unwrap()
        .replace_all(&markdown, "- $1\n")
        .to_string();

    // Remove remaining HTML tags
    markdown = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&markdown, "")
        .to_string();

    // Clean up whitespace
    markdown = regex::Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&markdown, "\n\n")
        .to_string();

    Ok(markdown.trim().to_string())
}

/// Convert HTML to plain text
pub fn html_to_plain_text(html: &str) -> String {
    // Remove script and style elements
    let text = regex::Regex::new(r"<(script|style)[^>]*>[\s\S]*?</\1>")
        .unwrap()
        .replace_all(html, "");

    // Add newlines for block elements
    let text = regex::Regex::new(r"</(p|div|h[1-6]|li|br)[^>]*>")
        .unwrap()
        .replace_all(&text, "\n");

    // Remove HTML tags
    let text = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&text, "");

    // Decode HTML entities
    let text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Clean up whitespace
    let text = regex::Regex::new(r"[ \t]+")
        .unwrap()
        .replace_all(&text, " ");
    let text = regex::Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&text, "\n\n");

    text.trim().to_string()
}

/// Convert Markdown to plain text
pub fn markdown_to_plain_text(markdown: &str) -> String {
    // First convert to HTML, then to plain text
    let html = markdown_to_html(markdown);
    html_to_plain_text(&html)
}

/// Add syntax highlighting classes to code blocks
pub fn highlight_code_blocks(html: &str) -> String {
    // This would integrate with a syntax highlighter like syntect
    // For now, just return the input
    html.to_string()
}

/// Extract headings from Markdown
pub fn extract_headings(markdown: &str) -> Vec<(u8, String)> {
    let mut headings = Vec::new();

    for line in markdown.lines() {
        if line.starts_with('#') {
            let level = line.chars().take_while(|c| *c == '#').count() as u8;
            let text = line.trim_start_matches('#').trim().to_string();
            headings.push((level, text));
        }
    }

    headings
}

/// Generate table of contents from Markdown
pub fn generate_toc(markdown: &str) -> String {
    let headings = extract_headings(markdown);
    let mut toc = String::new();

    for (level, text) in headings {
        let indent = "  ".repeat((level - 1) as usize);
        let slug = text.to_lowercase().replace(' ', "-");
        toc.push_str(&format!("{}- [{}](#{})\n", indent, text, slug));
    }

    toc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        let md = "# Hello\n\nThis is **bold** and *italic*.";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_html_to_plain_text() {
        let html = "<h1>Title</h1><p>This is a <strong>paragraph</strong>.</p>";
        let text = html_to_plain_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("paragraph"));
        assert!(!text.contains("<"));
    }

    #[test]
    fn test_extract_headings() {
        let md = "# Title\n## Section 1\n### Subsection\n## Section 2";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0], (1, "Title".to_string()));
        assert_eq!(headings[1], (2, "Section 1".to_string()));
    }
}
