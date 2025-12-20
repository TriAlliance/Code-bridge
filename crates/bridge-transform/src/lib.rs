//! File format transformation and conversion engine
//!
//! This crate provides:
//! - Image format conversion (PNG, JPEG, WebP, AVIF)
//! - Data format conversion (JSON, YAML, TOML)
//! - Document conversion (Markdown to HTML)
//! - Batch processing with caching

pub mod transformers;
pub mod registry;
pub mod pipeline;
pub mod cache;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Transformation failed: {0}")]
    Failed(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML error: {0}")]
    Toml(String),
}

pub type Result<T> = std::result::Result<T, TransformError>;

/// Supported file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Format {
    // Images
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Ico,
    Tiff,

    // Data
    Json,
    Yaml,
    Toml,
    Xml,
    Csv,

    // Documents
    Markdown,
    Html,
    PlainText,

    // Code
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl Format {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(Format::Png),
            "jpg" | "jpeg" => Some(Format::Jpeg),
            "webp" => Some(Format::WebP),
            "gif" => Some(Format::Gif),
            "bmp" => Some(Format::Bmp),
            "ico" => Some(Format::Ico),
            "tiff" | "tif" => Some(Format::Tiff),
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            "xml" => Some(Format::Xml),
            "csv" => Some(Format::Csv),
            "md" | "markdown" => Some(Format::Markdown),
            "html" | "htm" => Some(Format::Html),
            "txt" => Some(Format::PlainText),
            "rs" => Some(Format::Rust),
            "py" => Some(Format::Python),
            "js" => Some(Format::JavaScript),
            "ts" => Some(Format::TypeScript),
            _ => None,
        }
    }

    /// Get file extension for format
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::WebP => "webp",
            Format::Gif => "gif",
            Format::Bmp => "bmp",
            Format::Ico => "ico",
            Format::Tiff => "tiff",
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
            Format::Xml => "xml",
            Format::Csv => "csv",
            Format::Markdown => "md",
            Format::Html => "html",
            Format::PlainText => "txt",
            Format::Rust => "rs",
            Format::Python => "py",
            Format::JavaScript => "js",
            Format::TypeScript => "ts",
        }
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
            Format::WebP => "image/webp",
            Format::Gif => "image/gif",
            Format::Bmp => "image/bmp",
            Format::Ico => "image/x-icon",
            Format::Tiff => "image/tiff",
            Format::Json => "application/json",
            Format::Yaml => "application/yaml",
            Format::Toml => "application/toml",
            Format::Xml => "application/xml",
            Format::Csv => "text/csv",
            Format::Markdown => "text/markdown",
            Format::Html => "text/html",
            Format::PlainText => "text/plain",
            Format::Rust => "text/x-rust",
            Format::Python => "text/x-python",
            Format::JavaScript => "text/javascript",
            Format::TypeScript => "text/typescript",
        }
    }

    /// Check if format is an image
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Format::Png | Format::Jpeg | Format::WebP | Format::Gif |
            Format::Bmp | Format::Ico | Format::Tiff
        )
    }

    /// Check if format is a data format
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            Format::Json | Format::Yaml | Format::Toml | Format::Xml | Format::Csv
        )
    }

    /// Check if format is a document
    pub fn is_document(&self) -> bool {
        matches!(
            self,
            Format::Markdown | Format::Html | Format::PlainText
        )
    }
}

/// Transformation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOptions {
    /// Image quality (1-100)
    pub quality: Option<u8>,
    /// Resize dimensions
    pub resize: Option<(u32, u32)>,
    /// Preserve aspect ratio when resizing
    pub preserve_aspect: bool,
    /// Pretty print output
    pub pretty: bool,
    /// Include comments (for code)
    pub include_comments: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            quality: Some(85),
            resize: None,
            preserve_aspect: true,
            pretty: true,
            include_comments: true,
        }
    }
}

/// Transform a file from one format to another
pub fn transform_file(
    input_path: &PathBuf,
    output_path: &PathBuf,
    options: &TransformOptions,
) -> Result<()> {
    let input_ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| TransformError::UnsupportedFormat("No extension".to_string()))?;

    let output_ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| TransformError::UnsupportedFormat("No extension".to_string()))?;

    let input_format = Format::from_extension(input_ext)
        .ok_or_else(|| TransformError::UnsupportedFormat(input_ext.to_string()))?;

    let output_format = Format::from_extension(output_ext)
        .ok_or_else(|| TransformError::UnsupportedFormat(output_ext.to_string()))?;

    let input_data = std::fs::read(input_path)?;
    let output_data = transform(&input_data, input_format, output_format, options)?;
    std::fs::write(output_path, output_data)?;

    Ok(())
}

/// Transform data from one format to another
pub fn transform(
    data: &[u8],
    from: Format,
    to: Format,
    options: &TransformOptions,
) -> Result<Vec<u8>> {
    // Image transformations
    if from.is_image() && to.is_image() {
        return transformers::image::transform_image(data, from, to, options);
    }

    // Data format transformations
    if from.is_data() && to.is_data() {
        return transformers::data::transform_data(data, from, to, options);
    }

    // Document transformations
    if from.is_document() && to.is_document() {
        return transformers::document::transform_document(data, from, to, options);
    }

    Err(TransformError::UnsupportedFormat(format!(
        "Cannot transform {:?} to {:?}",
        from, to
    )))
}

/// Quick conversion functions
pub mod convert {
    use super::*;

    /// Convert JSON to YAML
    pub fn json_to_yaml(json: &str) -> Result<String> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let yaml = serde_yaml::to_string(&value)?;
        Ok(yaml)
    }

    /// Convert YAML to JSON
    pub fn yaml_to_json(yaml: &str, pretty: bool) -> Result<String> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
        let json = if pretty {
            serde_json::to_string_pretty(&value)?
        } else {
            serde_json::to_string(&value)?
        };
        Ok(json)
    }

    /// Convert JSON to TOML
    pub fn json_to_toml(json: &str) -> Result<String> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let toml = toml::to_string_pretty(&value)
            .map_err(|e| TransformError::Toml(e.to_string()))?;
        Ok(toml)
    }

    /// Convert TOML to JSON
    pub fn toml_to_json(toml_str: &str, pretty: bool) -> Result<String> {
        let value: toml::Value = toml::from_str(toml_str)
            .map_err(|e| TransformError::Toml(e.to_string()))?;
        let json = if pretty {
            serde_json::to_string_pretty(&value)?
        } else {
            serde_json::to_string(&value)?
        };
        Ok(json)
    }

    /// Convert Markdown to HTML
    pub fn markdown_to_html(markdown: &str) -> String {
        use pulldown_cmark::{html, Parser};

        let parser = Parser::new(markdown);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }

    /// Convert image to PNG
    pub fn to_png(data: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;
        let mut output = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        Ok(output)
    }

    /// Convert image to JPEG with quality
    pub fn to_jpeg(data: &[u8], quality: u8) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;
        let mut output = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
        img.write_with_encoder(encoder)?;
        Ok(output)
    }

    /// Convert image to WebP
    pub fn to_webp(data: &[u8], quality: u8) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;
        let rgba = img.to_rgba8();

        let encoder = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height());
        let encoded = encoder.encode(quality as f32);

        Ok(encoded.to_vec())
    }

    /// Resize image
    pub fn resize_image(data: &[u8], width: u32, height: u32, preserve_aspect: bool) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;

        let resized = if preserve_aspect {
            img.resize(width, height, image::imageops::FilterType::Lanczos3)
        } else {
            img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        };

        let mut output = Vec::new();
        resized.write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        Ok(output)
    }

    /// Create thumbnail
    pub fn create_thumbnail(data: &[u8], size: u32) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;
        let thumbnail = img.thumbnail(size, size);

        let mut output = Vec::new();
        thumbnail.write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(Format::from_extension("png"), Some(Format::Png));
        assert_eq!(Format::from_extension("JSON"), Some(Format::Json));
        assert_eq!(Format::from_extension("yml"), Some(Format::Yaml));
    }

    #[test]
    fn test_json_to_yaml() {
        let json = r#"{"name": "test", "value": 42}"#;
        let yaml = convert::json_to_yaml(json).unwrap();
        assert!(yaml.contains("name: test"));
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "# Hello\n\nThis is **bold**.";
        let html = convert::markdown_to_html(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }
}
