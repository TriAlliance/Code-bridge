//! Annotation tools for screenshots

use crate::{Annotation, Color, Point, Result, ScreenshotError, Size};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_circle_mut,
    draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use rusttype::{Font, Scale};
use std::path::Path;

/// Bounds for window/region
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Apply annotation to image file
pub fn apply_annotation(image_path: &Path, annotation: &Annotation) -> Result<()> {
    let img = image::open(image_path)?;
    let mut rgba = img.to_rgba8();

    apply_annotation_to_image(&mut rgba, annotation)?;

    rgba.save(image_path)?;
    Ok(())
}

/// Apply annotation to image in memory
pub fn apply_annotation_to_image(img: &mut RgbaImage, annotation: &Annotation) -> Result<()> {
    match annotation {
        Annotation::Arrow { start, end, color, width } => {
            draw_arrow(img, *start, *end, *color, *width);
        }
        Annotation::Rectangle { position, size, color, width, filled } => {
            draw_rectangle(img, *position, *size, *color, *width, *filled);
        }
        Annotation::Circle { center, radius, color, width, filled } => {
            draw_circle(img, *center, *radius, *color, *width, *filled);
        }
        Annotation::Text { position, content, color, font_size } => {
            draw_text(img, *position, content, *color, *font_size)?;
        }
        Annotation::Highlight { position, size, color, opacity } => {
            draw_highlight(img, *position, *size, *color, *opacity);
        }
        Annotation::Blur { position, size, intensity } => {
            draw_blur(img, *position, *size, *intensity);
        }
        Annotation::Number { position, number, color, size } => {
            draw_number(img, *position, *number, *color, *size)?;
        }
    }

    Ok(())
}

/// Apply multiple annotations
pub fn apply_annotations(image_path: &Path, annotations: &[Annotation]) -> Result<()> {
    let img = image::open(image_path)?;
    let mut rgba = img.to_rgba8();

    for annotation in annotations {
        apply_annotation_to_image(&mut rgba, annotation)?;
    }

    rgba.save(image_path)?;
    Ok(())
}

fn color_to_rgba(color: Color) -> Rgba<u8> {
    Rgba([color.r, color.g, color.b, color.a])
}

fn draw_arrow(img: &mut RgbaImage, start: Point, end: Point, color: Color, width: f32) {
    let rgba = color_to_rgba(color);

    // Draw main line
    draw_line_segment_mut(
        img,
        (start.x, start.y),
        (end.x, end.y),
        rgba,
    );

    // Draw additional lines for width
    for i in 1..=(width as i32 / 2) {
        let offset = i as f32;
        draw_line_segment_mut(
            img,
            (start.x + offset, start.y),
            (end.x + offset, end.y),
            rgba,
        );
        draw_line_segment_mut(
            img,
            (start.x - offset, start.y),
            (end.x - offset, end.y),
            rgba,
        );
        draw_line_segment_mut(
            img,
            (start.x, start.y + offset),
            (end.x, end.y + offset),
            rgba,
        );
        draw_line_segment_mut(
            img,
            (start.x, start.y - offset),
            (end.x, end.y - offset),
            rgba,
        );
    }

    // Draw arrowhead
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();

    if length > 0.0 {
        let arrow_length = 15.0;
        let arrow_width = 8.0;

        let ux = dx / length;
        let uy = dy / length;

        let px = -uy;
        let py = ux;

        let arrow_base_x = end.x - ux * arrow_length;
        let arrow_base_y = end.y - uy * arrow_length;

        let left_x = arrow_base_x + px * arrow_width;
        let left_y = arrow_base_y + py * arrow_width;

        let right_x = arrow_base_x - px * arrow_width;
        let right_y = arrow_base_y - py * arrow_width;

        draw_line_segment_mut(img, (end.x, end.y), (left_x, left_y), rgba);
        draw_line_segment_mut(img, (end.x, end.y), (right_x, right_y), rgba);
    }
}

fn draw_rectangle(
    img: &mut RgbaImage,
    position: Point,
    size: Size,
    color: Color,
    width: f32,
    filled: bool,
) {
    let rgba = color_to_rgba(color);
    let rect = Rect::at(position.x as i32, position.y as i32)
        .of_size(size.width as u32, size.height as u32);

    if filled {
        draw_filled_rect_mut(img, rect, rgba);
    } else {
        // Draw multiple rectangles for border width
        for i in 0..=(width as i32) {
            let r = Rect::at(position.x as i32 + i, position.y as i32 + i)
                .of_size(
                    (size.width as i32 - 2 * i).max(1) as u32,
                    (size.height as i32 - 2 * i).max(1) as u32,
                );
            draw_hollow_rect_mut(img, r, rgba);
        }
    }
}

fn draw_circle(
    img: &mut RgbaImage,
    center: Point,
    radius: f32,
    color: Color,
    _width: f32,
    filled: bool,
) {
    let rgba = color_to_rgba(color);

    if filled {
        draw_filled_circle_mut(
            img,
            (center.x as i32, center.y as i32),
            radius as i32,
            rgba,
        );
    } else {
        draw_hollow_circle_mut(
            img,
            (center.x as i32, center.y as i32),
            radius as i32,
            rgba,
        );
    }
}

fn draw_text(
    img: &mut RgbaImage,
    position: Point,
    content: &str,
    color: Color,
    font_size: f32,
) -> Result<()> {
    let rgba = color_to_rgba(color);

    // Use a built-in font (DejaVu Sans Mono is commonly available)
    let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8])
        .ok_or_else(|| ScreenshotError::AnnotationFailed("Failed to load font".to_string()))?;

    let scale = Scale::uniform(font_size);

    draw_text_mut(
        img,
        rgba,
        position.x as i32,
        position.y as i32,
        scale,
        &font,
        content,
    );

    Ok(())
}

fn draw_highlight(img: &mut RgbaImage, position: Point, size: Size, color: Color, opacity: f32) {
    let alpha = (opacity * 255.0) as u8;
    let highlight_color = Rgba([color.r, color.g, color.b, alpha]);

    for y in position.y as u32..(position.y + size.height) as u32 {
        for x in position.x as u32..(position.x + size.width) as u32 {
            if x < img.width() && y < img.height() {
                let pixel = img.get_pixel(x, y);
                // Blend with highlight color
                let blended = blend_pixel(*pixel, highlight_color);
                img.put_pixel(x, y, blended);
            }
        }
    }
}

fn blend_pixel(base: Rgba<u8>, overlay: Rgba<u8>) -> Rgba<u8> {
    let alpha = overlay[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    Rgba([
        (overlay[0] as f32 * alpha + base[0] as f32 * inv_alpha) as u8,
        (overlay[1] as f32 * alpha + base[1] as f32 * inv_alpha) as u8,
        (overlay[2] as f32 * alpha + base[2] as f32 * inv_alpha) as u8,
        255,
    ])
}

fn draw_blur(img: &mut RgbaImage, position: Point, size: Size, intensity: f32) {
    let blur_radius = (intensity * 10.0) as u32;
    let x_start = position.x as u32;
    let y_start = position.y as u32;
    let x_end = (position.x + size.width) as u32;
    let y_end = (position.y + size.height) as u32;

    // Simple box blur implementation
    for y in y_start..y_end.min(img.height()) {
        for x in x_start..x_end.min(img.width()) {
            let mut r_sum: u32 = 0;
            let mut g_sum: u32 = 0;
            let mut b_sum: u32 = 0;
            let mut count: u32 = 0;

            for dy in 0..blur_radius {
                for dx in 0..blur_radius {
                    let nx = x.saturating_add(dx).saturating_sub(blur_radius / 2);
                    let ny = y.saturating_add(dy).saturating_sub(blur_radius / 2);

                    if nx < img.width() && ny < img.height() {
                        let pixel = img.get_pixel(nx, ny);
                        r_sum += pixel[0] as u32;
                        g_sum += pixel[1] as u32;
                        b_sum += pixel[2] as u32;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let blurred = Rgba([
                    (r_sum / count) as u8,
                    (g_sum / count) as u8,
                    (b_sum / count) as u8,
                    255,
                ]);
                img.put_pixel(x, y, blurred);
            }
        }
    }
}

fn draw_number(
    img: &mut RgbaImage,
    position: Point,
    number: u32,
    color: Color,
    size: f32,
) -> Result<()> {
    // Draw circle background
    let radius = size / 2.0;
    draw_filled_circle_mut(
        img,
        (position.x as i32 + radius as i32, position.y as i32 + radius as i32),
        radius as i32,
        color_to_rgba(color),
    );

    // Draw number text in white
    let text_color = if color.r + color.g + color.b > 382 {
        Color::BLACK
    } else {
        Color::WHITE
    };

    let font_size = size * 0.6;
    let text_x = position.x + radius - font_size / 4.0;
    let text_y = position.y + radius - font_size / 2.0;

    draw_text(
        img,
        Point { x: text_x, y: text_y },
        &number.to_string(),
        text_color,
        font_size,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_rgba() {
        let color = Color::RED;
        let rgba = color_to_rgba(color);
        assert_eq!(rgba[0], 255);
        assert_eq!(rgba[1], 59);
        assert_eq!(rgba[2], 48);
        assert_eq!(rgba[3], 255);
    }
}
