//! Platform-specific clipboard implementations

use crate::{ClipboardEntry, ClipboardError, ContentType, Result};
use arboard::Clipboard;

/// Get current clipboard content
pub fn get_clipboard_content(device_id: &str) -> Result<Option<ClipboardEntry>> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;

    // Try to get text first
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            // Detect if it's a URL
            if text.starts_with("http://") || text.starts_with("https://") {
                return Ok(Some(ClipboardEntry::new_url(text, device_id.to_string())));
            }

            // Detect if it's code (simple heuristics)
            if looks_like_code(&text) {
                let language = detect_language(&text);
                return Ok(Some(ClipboardEntry::new_code(
                    text,
                    language,
                    device_id.to_string(),
                )));
            }

            return Ok(Some(ClipboardEntry::new_text(text, device_id.to_string())));
        }
    }

    // Try to get image
    if let Ok(image) = clipboard.get_image() {
        let rgba_data = image.bytes.to_vec();
        return Ok(Some(ClipboardEntry::new_image(
            rgba_data,
            device_id.to_string(),
        )));
    }

    Ok(None)
}

/// Set clipboard content
pub fn set_clipboard_content(entry: &ClipboardEntry) -> Result<()> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;

    match &entry.content_type {
        ContentType::Text | ContentType::RichText | ContentType::Code { .. } | ContentType::Url => {
            let text = String::from_utf8(entry.data.clone())
                .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;
            clipboard.set_text(text)
                .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;
        }
        ContentType::Image => {
            // For images, we need to decode the data
            if let Ok(img) = image::load_from_memory(&entry.data) {
                let rgba = img.to_rgba8();
                let image_data = arboard::ImageData {
                    width: rgba.width() as usize,
                    height: rgba.height() as usize,
                    bytes: rgba.into_raw().into(),
                };
                clipboard.set_image(image_data)
                    .map_err(|e| ClipboardError::AccessFailed(e.to_string()))?;
            }
        }
        ContentType::Files => {
            // File handling would require platform-specific implementation
            return Err(ClipboardError::UnsupportedType("Files".to_string()));
        }
    }

    Ok(())
}

/// Simple heuristics to detect if text looks like code
fn looks_like_code(text: &str) -> bool {
    let code_indicators = [
        "fn ", "function ", "def ", "class ", "import ", "export ",
        "const ", "let ", "var ", "pub ", "struct ", "enum ",
        "if (", "if(", "for (", "for(", "while (", "while(",
        "return ", "async ", "await ", "impl ", "trait ",
        "package ", "namespace ", "using ", "include ",
        "<?php", "<?=", "#!/", "SELECT ", "INSERT ", "UPDATE ",
    ];

    let bracket_count = text.matches('{').count() + text.matches('}').count();
    let paren_count = text.matches('(').count() + text.matches(')').count();
    let semicolon_count = text.matches(';').count();

    // Has code keywords
    let has_keywords = code_indicators.iter().any(|k| text.contains(k));

    // Has significant brackets/parens
    let has_syntax = bracket_count >= 2 || (paren_count >= 2 && semicolon_count >= 1);

    has_keywords || has_syntax
}

/// Detect programming language from code
fn detect_language(text: &str) -> String {
    let patterns = [
        ("rust", vec!["fn ", "let mut", "impl ", "pub fn", "use std::", "&str", "-> Result"]),
        ("python", vec!["def ", "import ", "from ", "class ", "self.", "__init__", "elif "]),
        ("javascript", vec!["const ", "let ", "function ", "=> ", "async function", "require("]),
        ("typescript", vec!["interface ", ": string", ": number", ": boolean", "as const"]),
        ("go", vec!["func ", "package ", "import (", "go func", "chan ", "defer "]),
        ("java", vec!["public class", "private ", "protected ", "void ", "System.out"]),
        ("cpp", vec!["#include", "std::", "cout", "cin", "nullptr", "template<"]),
        ("c", vec!["#include", "printf", "scanf", "malloc", "typedef struct"]),
        ("swift", vec!["import SwiftUI", "var body", "func ", "@State", "@Binding"]),
        ("ruby", vec!["def ", "end", "puts ", "require ", "attr_accessor"]),
        ("php", vec!["<?php", "<?=", "function ", "echo ", "$this->", "namespace "]),
        ("sql", vec!["SELECT ", "INSERT ", "UPDATE ", "DELETE ", "FROM ", "WHERE "]),
        ("shell", vec!["#!/bin/", "echo ", "if [", "fi", "done", "export "]),
        ("yaml", vec!["- name:", "  - ", "  key:", "---"]),
        ("json", vec![r#"":"#, "{\n  ", "[\n  "]),
    ];

    for (lang, indicators) in patterns {
        let matches = indicators.iter().filter(|i| text.contains(*i)).count();
        if matches >= 2 || (matches == 1 && text.len() < 500) {
            return lang.to_string();
        }
    }

    "text".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_code() {
        assert!(looks_like_code("fn main() { println!(\"Hello\"); }"));
        assert!(looks_like_code("function test() { return 42; }"));
        assert!(looks_like_code("def hello(): print('world')"));
        assert!(!looks_like_code("Hello, World!"));
        assert!(!looks_like_code("Just a regular sentence."));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("fn main() { let x = 5; }"), "rust");
        assert_eq!(detect_language("def hello(): print('hi')"), "python");
        assert_eq!(detect_language("const x = () => {}"), "javascript");
    }
}
