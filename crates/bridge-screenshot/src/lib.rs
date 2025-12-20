//! Screenshot capture, OCR, and annotation for Code Bridge
//!
//! This crate provides cross-platform screenshot functionality with:
//! - Region, window, and full-screen capture
//! - OCR text extraction
//! - Annotation tools (arrows, boxes, text)
//! - Automatic sync to QNAP/P2P

pub mod capture;
pub mod ocr;
pub mod annotation;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ScreenshotError {
    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("OCR failed: {0}")]
    OcrFailed(String),

    #[error("Annotation failed: {0}")]
    AnnotationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Storage error: {0}")]
    Storage(String),
}

pub type Result<T> = std::result::Result<T, ScreenshotError>;

/// Screenshot capture mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureMode {
    /// Full screen capture
    FullScreen,
    /// Specific region
    Region {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// Specific window
    Window { window_id: String },
    /// Interactive selection
    Interactive,
}

/// Screenshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub captured_at: DateTime<Utc>,
    pub source_device: String,
    pub ocr_text: Option<String>,
    pub annotations: Vec<Annotation>,
    pub tags: Vec<String>,
    pub content_hash: String,
}

impl Screenshot {
    pub fn new(path: PathBuf, width: u32, height: u32) -> Self {
        let id = Uuid::new_v4().to_string();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("screenshot_{}", id));

        Self {
            id,
            name,
            path,
            width,
            height,
            format: ImageFormat::Png,
            captured_at: Utc::now(),
            source_device: String::new(),
            ocr_text: None,
            annotations: Vec::new(),
            tags: Vec::new(),
            content_hash: String::new(),
        }
    }

    pub fn with_ocr(mut self, text: String) -> Self {
        self.ocr_text = Some(text);
        self
    }

    pub fn with_device(mut self, device: String) -> Self {
        self.source_device = device;
        self
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.content_hash = hash;
        self
    }
}

/// Supported image formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Avif,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => "webp",
            ImageFormat::Avif => "avif",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Avif => "image/avif",
        }
    }
}

/// Annotation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Annotation {
    Arrow {
        start: Point,
        end: Point,
        color: Color,
        width: f32,
    },
    Rectangle {
        position: Point,
        size: Size,
        color: Color,
        width: f32,
        filled: bool,
    },
    Circle {
        center: Point,
        radius: f32,
        color: Color,
        width: f32,
        filled: bool,
    },
    Text {
        position: Point,
        content: String,
        color: Color,
        font_size: f32,
    },
    Highlight {
        position: Point,
        size: Size,
        color: Color,
        opacity: f32,
    },
    Blur {
        position: Point,
        size: Size,
        intensity: f32,
    },
    Number {
        position: Point,
        number: u32,
        color: Color,
        size: f32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const RED: Color = Color { r: 255, g: 59, b: 48, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 122, b: 255, a: 255 };
    pub const GREEN: Color = Color { r: 52, g: 199, b: 89, a: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 204, b: 0, a: 255 };
    pub const ORANGE: Color = Color { r: 255, g: 149, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
}

/// Screenshot manager for capture and storage
pub struct ScreenshotManager {
    storage_path: PathBuf,
    device_id: String,
    default_format: ImageFormat,
}

impl ScreenshotManager {
    pub fn new(storage_path: PathBuf, device_id: String) -> Self {
        Self {
            storage_path,
            device_id,
            default_format: ImageFormat::Png,
        }
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.default_format = format;
        self
    }

    /// Capture a screenshot
    pub async fn capture(&self, mode: CaptureMode) -> Result<Screenshot> {
        let image_data = capture::capture_screen(mode).await?;

        // Generate filename
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("screenshot_{}.{}", timestamp, self.default_format.extension());
        let path = self.storage_path.join(&filename);

        // Save image
        std::fs::create_dir_all(&self.storage_path)?;
        image_data.save(&path)?;

        // Calculate hash
        let hash = bridge_core::storage::ContentStore::hash(&std::fs::read(&path)?);

        let screenshot = Screenshot::new(path, image_data.width(), image_data.height())
            .with_device(self.device_id.clone())
            .with_hash(hash);

        Ok(screenshot)
    }

    /// Capture with OCR
    pub async fn capture_with_ocr(&self, mode: CaptureMode) -> Result<Screenshot> {
        let mut screenshot = self.capture(mode).await?;

        // Perform OCR
        let text = ocr::extract_text(&screenshot.path).await?;
        screenshot.ocr_text = Some(text);

        Ok(screenshot)
    }

    /// Add annotation to screenshot
    pub fn annotate(&self, screenshot: &mut Screenshot, annotation: Annotation) -> Result<()> {
        annotation::apply_annotation(&screenshot.path, &annotation)?;
        screenshot.annotations.push(annotation);
        Ok(())
    }

    /// List all screenshots
    pub fn list_screenshots(&self) -> Result<Vec<Screenshot>> {
        let mut screenshots = Vec::new();

        if self.storage_path.exists() {
            for entry in std::fs::read_dir(&self.storage_path)? {
                let entry = entry?;
                let path = entry.path();

                if let Some(ext) = path.extension() {
                    if ["png", "jpg", "jpeg", "webp", "avif"].contains(&ext.to_str().unwrap_or("")) {
                        if let Ok(img) = image::open(&path) {
                            let screenshot = Screenshot::new(path, img.width(), img.height())
                                .with_device(self.device_id.clone());
                            screenshots.push(screenshot);
                        }
                    }
                }
            }
        }

        // Sort by capture time (newest first)
        screenshots.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));

        Ok(screenshots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_screenshot_creation() {
        let screenshot = Screenshot::new(
            PathBuf::from("/tmp/test.png"),
            1920,
            1080,
        );

        assert!(!screenshot.id.is_empty());
        assert_eq!(screenshot.width, 1920);
        assert_eq!(screenshot.height, 1080);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::RED.r, 255);
        assert_eq!(Color::BLUE.b, 255);
    }
}
