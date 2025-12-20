//! Screenshot capture implementations

use crate::{CaptureMode, Result, ScreenshotError};
use image::DynamicImage;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

/// Capture screen based on mode
pub async fn capture_screen(mode: CaptureMode) -> Result<DynamicImage> {
    #[cfg(target_os = "linux")]
    {
        linux::capture(mode).await
    }

    #[cfg(target_os = "macos")]
    {
        macos::capture(mode).await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(ScreenshotError::CaptureFailed(
            "Platform not supported".to_string(),
        ))
    }
}

/// Get list of available windows
pub async fn list_windows() -> Result<Vec<WindowInfo>> {
    #[cfg(target_os = "linux")]
    {
        linux::list_windows().await
    }

    #[cfg(target_os = "macos")]
    {
        macos::list_windows().await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(Vec::new())
    }
}

/// Window information
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub bounds: crate::annotation::Bounds,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub async fn capture(mode: CaptureMode) -> Result<DynamicImage> {
        use screenshots::Screen;

        match mode {
            CaptureMode::FullScreen => {
                let screens = Screen::all()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if let Some(screen) = screens.first() {
                    let image = screen
                        .capture()
                        .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                    let img = image::RgbaImage::from_raw(
                        image.width(),
                        image.height(),
                        image.to_vec(),
                    )
                    .ok_or_else(|| ScreenshotError::CaptureFailed("Failed to create image".to_string()))?;

                    Ok(DynamicImage::ImageRgba8(img))
                } else {
                    Err(ScreenshotError::CaptureFailed("No screens found".to_string()))
                }
            }
            CaptureMode::Region { x, y, width, height } => {
                let screens = Screen::all()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if let Some(screen) = screens.first() {
                    let image = screen
                        .capture_area(x, y, width, height)
                        .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                    let img = image::RgbaImage::from_raw(
                        image.width(),
                        image.height(),
                        image.to_vec(),
                    )
                    .ok_or_else(|| ScreenshotError::CaptureFailed("Failed to create image".to_string()))?;

                    Ok(DynamicImage::ImageRgba8(img))
                } else {
                    Err(ScreenshotError::CaptureFailed("No screens found".to_string()))
                }
            }
            CaptureMode::Window { window_id } => {
                // Use xdotool or similar to get window geometry
                // Then capture that region
                Err(ScreenshotError::CaptureFailed(
                    format!("Window capture not implemented for window {}", window_id),
                ))
            }
            CaptureMode::Interactive => {
                // Would need to launch a selection UI
                Err(ScreenshotError::CaptureFailed(
                    "Interactive mode requires GUI".to_string(),
                ))
            }
        }
    }

    pub async fn list_windows() -> Result<Vec<WindowInfo>> {
        // Use wmctrl or xdotool to list windows
        Ok(Vec::new())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    pub async fn capture(mode: CaptureMode) -> Result<DynamicImage> {
        use std::process::Command;

        match mode {
            CaptureMode::FullScreen => {
                let temp_path = format!("/tmp/screenshot_{}.png", uuid::Uuid::new_v4());

                let status = Command::new("screencapture")
                    .args(["-x", &temp_path])
                    .status()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if !status.success() {
                    return Err(ScreenshotError::CaptureFailed(
                        "screencapture failed".to_string(),
                    ));
                }

                let img = image::open(&temp_path)?;
                std::fs::remove_file(&temp_path).ok();

                Ok(img)
            }
            CaptureMode::Region { x, y, width, height } => {
                let temp_path = format!("/tmp/screenshot_{}.png", uuid::Uuid::new_v4());
                let rect = format!("{},{},{},{}", x, y, width, height);

                let status = Command::new("screencapture")
                    .args(["-x", "-R", &rect, &temp_path])
                    .status()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if !status.success() {
                    return Err(ScreenshotError::CaptureFailed(
                        "screencapture failed".to_string(),
                    ));
                }

                let img = image::open(&temp_path)?;
                std::fs::remove_file(&temp_path).ok();

                Ok(img)
            }
            CaptureMode::Window { window_id } => {
                let temp_path = format!("/tmp/screenshot_{}.png", uuid::Uuid::new_v4());

                let status = Command::new("screencapture")
                    .args(["-x", "-l", &window_id, &temp_path])
                    .status()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if !status.success() {
                    return Err(ScreenshotError::CaptureFailed(
                        "screencapture failed".to_string(),
                    ));
                }

                let img = image::open(&temp_path)?;
                std::fs::remove_file(&temp_path).ok();

                Ok(img)
            }
            CaptureMode::Interactive => {
                let temp_path = format!("/tmp/screenshot_{}.png", uuid::Uuid::new_v4());

                let status = Command::new("screencapture")
                    .args(["-i", &temp_path])
                    .status()
                    .map_err(|e| ScreenshotError::CaptureFailed(e.to_string()))?;

                if !status.success() {
                    return Err(ScreenshotError::CaptureFailed(
                        "screencapture cancelled or failed".to_string(),
                    ));
                }

                let img = image::open(&temp_path)?;
                std::fs::remove_file(&temp_path).ok();

                Ok(img)
            }
        }
    }

    pub async fn list_windows() -> Result<Vec<WindowInfo>> {
        // Use CoreGraphics to list windows
        Ok(Vec::new())
    }
}
