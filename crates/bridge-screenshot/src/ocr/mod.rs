//! OCR text extraction from screenshots

use crate::{Result, ScreenshotError};
use std::path::Path;
use std::process::Command;

/// Extract text from an image using OCR
pub async fn extract_text(image_path: &Path) -> Result<String> {
    // Try tesseract first
    if let Ok(text) = extract_with_tesseract(image_path).await {
        return Ok(text);
    }

    // Fallback to system-specific OCR
    #[cfg(target_os = "macos")]
    {
        extract_with_vision(image_path).await
    }

    #[cfg(target_os = "linux")]
    {
        Err(ScreenshotError::OcrFailed(
            "Tesseract not available. Install with: sudo apt install tesseract-ocr".to_string(),
        ))
    }
}

/// Extract text using Tesseract OCR
async fn extract_with_tesseract(image_path: &Path) -> Result<String> {
    let output = Command::new("tesseract")
        .args([
            image_path.to_str().unwrap(),
            "stdout",
            "-l", "eng",
            "--psm", "3",
        ])
        .output()
        .map_err(|e| ScreenshotError::OcrFailed(format!("Failed to run tesseract: {}", e)))?;

    if !output.status.success() {
        return Err(ScreenshotError::OcrFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text)
}

/// Extract text using macOS Vision framework
#[cfg(target_os = "macos")]
async fn extract_with_vision(image_path: &Path) -> Result<String> {
    use std::process::Command;

    // Use osascript to call Vision framework
    let script = format!(
        r#"
        use framework "Vision"
        use framework "Foundation"
        use scripting additions

        set imagePath to "{}"
        set theImage to current application's NSImage's alloc()'s initWithContentsOfFile:imagePath

        if theImage is missing value then
            return ""
        end if

        set requestHandler to current application's VNImageRequestHandler's alloc()'s initWithData:(theImage's TIFFRepresentation()) options:(current application's NSDictionary's dictionary())

        set textRequest to current application's VNRecognizeTextRequest's alloc()'s init()
        textRequest's setRecognitionLevel:(current application's VNRequestTextRecognitionLevelAccurate)

        requestHandler's performRequests:(current application's NSArray's arrayWithObject:textRequest) |error|:(missing value)

        set results to textRequest's results()
        set outputText to ""

        repeat with observation in results
            set outputText to outputText & (observation's topCandidates:1's firstObject()'s |string|() as text) & linefeed
        end repeat

        return outputText
        "#,
        image_path.display()
    );

    let output = Command::new("osascript")
        .args(["-l", "AppleScript", "-e", &script])
        .output()
        .map_err(|e| ScreenshotError::OcrFailed(format!("Failed to run Vision OCR: {}", e)))?;

    if !output.status.success() {
        return Err(ScreenshotError::OcrFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text)
}

/// OCR configuration options
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Language for OCR (e.g., "eng", "deu", "fra")
    pub language: String,
    /// Page segmentation mode
    pub psm: PageSegmentationMode,
    /// Whitelist of characters to recognize
    pub whitelist: Option<String>,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: PageSegmentationMode::Auto,
            whitelist: None,
        }
    }
}

/// Tesseract page segmentation modes
#[derive(Debug, Clone, Copy)]
pub enum PageSegmentationMode {
    /// Orientation and script detection only
    OsdOnly = 0,
    /// Automatic page segmentation with OSD
    AutoOsd = 1,
    /// Automatic page segmentation, but no OSD
    AutoNoOsd = 2,
    /// Fully automatic page segmentation (default)
    Auto = 3,
    /// Assume a single column of text
    SingleColumn = 4,
    /// Assume a single uniform block of vertically aligned text
    SingleBlockVertical = 5,
    /// Assume a single uniform block of text
    SingleBlock = 6,
    /// Treat the image as a single line
    SingleLine = 7,
    /// Treat the image as a single word
    SingleWord = 8,
    /// Treat the image as a single word in a circle
    CircleWord = 9,
    /// Treat the image as a single character
    SingleChar = 10,
    /// Sparse text
    SparseText = 11,
    /// Sparse text with OSD
    SparseTextOsd = 12,
    /// Raw line
    RawLine = 13,
}

/// Extract text with custom configuration
pub async fn extract_text_with_config(image_path: &Path, config: OcrConfig) -> Result<String> {
    let mut args = vec![
        image_path.to_str().unwrap().to_string(),
        "stdout".to_string(),
        "-l".to_string(),
        config.language,
        "--psm".to_string(),
        (config.psm as u8).to_string(),
    ];

    if let Some(whitelist) = config.whitelist {
        args.push("-c".to_string());
        args.push(format!("tessedit_char_whitelist={}", whitelist));
    }

    let output = Command::new("tesseract")
        .args(&args)
        .output()
        .map_err(|e| ScreenshotError::OcrFailed(format!("Failed to run tesseract: {}", e)))?;

    if !output.status.success() {
        return Err(ScreenshotError::OcrFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.language, "eng");
    }
}
