use crate::entity::Vertex;
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/default.ttf");

#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub width: f32,
    pub height: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub advance_width: f32,
}

/// Font atlas rasterizing TrueType glyphs into an R8 alpha texture.
pub struct FontAtlas {
    pub font: fontdue::Font,
    pub glyphs: HashMap<char, Glyph>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_data: Vec<u8>,
    pub base_size: f32,
    pub line_height: f32,
}

impl FontAtlas {
    /// Builds a new FontAtlas from font bytes (defaults to 48px rasterization for crisp sharpness).
    pub fn new(font_bytes: &[u8], base_size: f32) -> Result<Self, &'static str> {
        let settings = fontdue::FontSettings::default();
        let font = fontdue::Font::from_bytes(font_bytes, settings).map_err(|_| "Failed to parse TTF font")?;

        let atlas_width = 512;
        let atlas_height = 512;
        let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut glyphs = HashMap::new();

        let padding = 2;
        let mut cur_y = padding;
        let mut row_max_h = 0;

        // Reserve (0,0) as a 2x2 solid white square for solid non-text quads if needed
        for y in 0..2 {
            for x in 0..2 {
                atlas_data[(y * atlas_width + x) as usize] = 255;
            }
        }
        let mut cur_x = 4;

        // Rasterize standard printable ASCII (32 ' ' through 126 '~')
        for c in 32u8..=126u8 {
            let ch = c as char;
            let (metrics, bitmap) = font.rasterize(ch, base_size);

            let gw = metrics.width as u32;
            let gh = metrics.height as u32;

            if cur_x + gw + padding >= atlas_width {
                cur_x = padding;
                cur_y += row_max_h + padding;
                row_max_h = 0;
            }

            if cur_y + gh + padding >= atlas_height {
                eprintln!("[azterisk_render warning] Font atlas exceeded dimensions at char '{ch}'");
                break;
            }

            // Copy bitmap into atlas
            for row in 0..gh {
                for col in 0..gw {
                    let src_idx = (row * gw + col) as usize;
                    let dst_idx = ((cur_y + row) * atlas_width + (cur_x + col)) as usize;
                    atlas_data[dst_idx] = bitmap[src_idx];
                }
            }

            let uv_min = Vec2::new(
                cur_x as f32 / atlas_width as f32,
                cur_y as f32 / atlas_height as f32,
            );
            let uv_max = Vec2::new(
                (cur_x + gw) as f32 / atlas_width as f32,
                (cur_y + gh) as f32 / atlas_height as f32,
            );

            glyphs.insert(
                ch,
                Glyph {
                    uv_min,
                    uv_max,
                    width: metrics.width as f32,
                    height: metrics.height as f32,
                    x_offset: metrics.xmin as f32,
                    y_offset: metrics.ymin as f32,
                    advance_width: metrics.advance_width,
                },
            );

            cur_x += gw + padding;
            row_max_h = row_max_h.max(gh);
        }

        let line_height = font
            .horizontal_line_metrics(base_size)
            .map(|m| m.new_line_size)
            .unwrap_or(base_size * 1.2);

        Ok(Self {
            font,
            glyphs,
            atlas_width,
            atlas_height,
            atlas_data,
            base_size,
            line_height,
        })
    }

    /// Measures the total pixel width and height of a string at target font size.
    pub fn measure_text(&self, text: &str, target_size: f32) -> Vec2 {
        let scale = target_size / self.base_size;
        let mut width = 0.0f32;
        let height = self.line_height * scale;

        for ch in text.chars() {
            if let Some(g) = self.glyphs.get(&ch) {
                width += g.advance_width * scale;
            } else if ch == ' ' {
                width += (self.base_size * 0.3) * scale;
            }
        }

        Vec2::new(width, height)
    }

    /// Generates 2D quad vertices and indices for a string of text in screen coordinates.
    pub fn layout_text(
        &self,
        text: &str,
        origin_x: f32,
        baseline_y: f32,
        target_size: f32,
        color: Vec4,
        start_index: u32,
    ) -> (Vec<Vertex>, Vec<u32>) {
        let scale = target_size / self.base_size;
        let mut pen_x = origin_x;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let c = [color.x, color.y, color.z, color.w];
        let normal = [0.0, 0.0, 1.0];

        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                let gw = glyph.width * scale;
                let gh = glyph.height * scale;

                let left = pen_x + glyph.x_offset * scale;
                let right = left + gw;
                // In orthographic screen space: higher Y is up, baseline is zero
                let bottom = baseline_y + glyph.y_offset * scale;
                let top = bottom + gh;

                if gw > 0.0 && gh > 0.0 {
                    let base = start_index + vertices.len() as u32;

                    vertices.push(Vertex {
                        position: [left, bottom, 0.0],
                        normal,
                        uv: [glyph.uv_min.x, glyph.uv_max.y],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [right, bottom, 0.0],
                        normal,
                        uv: [glyph.uv_max.x, glyph.uv_max.y],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [right, top, 0.0],
                        normal,
                        uv: [glyph.uv_max.x, glyph.uv_min.y],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [left, top, 0.0],
                        normal,
                        uv: [glyph.uv_min.x, glyph.uv_min.y],
                        color: c,
                    });

                    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }

                pen_x += glyph.advance_width * scale;
            } else if ch == ' ' {
                pen_x += (self.base_size * 0.3) * scale;
            }
        }

        (vertices, indices)
    }
}
