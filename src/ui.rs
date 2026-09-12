use crate::entity::{Mesh, Vertex};
use crate::lighting::IntoColor;
use crate::text::FontAtlas;
use glam::{Vec2, Vec4};

/// Screen anchor position for responsive UI layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    /// Computes base screen pixel coordinates for the anchor.
    pub fn compute_origin(&self, screen_w: f32, screen_h: f32) -> Vec2 {
        match self {
            Anchor::TopLeft => Vec2::new(0.0, 0.0),
            Anchor::TopCenter => Vec2::new(screen_w / 2.0, 0.0),
            Anchor::TopRight => Vec2::new(screen_w, 0.0),
            Anchor::CenterLeft => Vec2::new(0.0, screen_h / 2.0),
            Anchor::Center => Vec2::new(screen_w / 2.0, screen_h / 2.0),
            Anchor::CenterRight => Vec2::new(screen_w, screen_h / 2.0),
            Anchor::BottomLeft => Vec2::new(0.0, screen_h),
            Anchor::BottomCenter => Vec2::new(screen_w / 2.0, screen_h),
            Anchor::BottomRight => Vec2::new(screen_w, screen_h),
        }
    }
}

/// Retained UI Component node supporting responsive anchors, borders, colors, text, and click sounds.
pub struct UIElement {
    pub id: String,
    pub anchor: Anchor,
    pub offset: Vec2,
    pub size: Vec2,

    pub color: Vec4,
    pub hover_color: Option<Vec4>,
    pub border_width: f32,
    pub border_color: Vec4,
    pub radius: f32,

    pub text: Option<String>,
    pub text_color: Vec4,
    pub text_size: f32,

    pub sound_path: Option<String>,
    pub is_hovered: bool,
    pub on_click: Option<Box<dyn FnMut(&mut UIElement) + Send + Sync>>,
}

impl UIElement {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            anchor: Anchor::TopLeft,
            offset: Vec2::ZERO,
            size: Vec2::new(100.0, 40.0),
            color: Vec4::new(0.12, 0.14, 0.18, 0.9), // Dark translucent default
            hover_color: None,
            border_width: 0.0,
            border_color: Vec4::ONE,
            radius: 0.0,
            text: None,
            text_color: Vec4::ONE,
            text_size: 16.0,
            sound_path: None,
            is_hovered: false,
            on_click: None,
        }
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Vec2::new(x, y);
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Vec2::new(width.max(1.0), height.max(1.0));
        self
    }

    pub fn color(mut self, hex_or_rgb: impl IntoColor) -> Self {
        let rgb = hex_or_rgb.into_color();
        self.color = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        self
    }

    pub fn hover_color(mut self, hex_or_rgb: impl IntoColor) -> Self {
        let rgb = hex_or_rgb.into_color();
        self.hover_color = Some(Vec4::new(rgb.x, rgb.y, rgb.z, 1.0));
        self
    }

    pub fn border(mut self, width: f32, color: impl IntoColor) -> Self {
        self.border_width = width.max(0.0);
        let rgb = color.into_color();
        self.border_color = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        self
    }

    /// Corner rounding radius (shared naming convention).
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    /// Sound effect triggered when the element is clicked (shared naming convention).
    pub fn sound(mut self, path: impl Into<String>) -> Self {
        self.sound_path = Some(path.into());
        self
    }

    pub fn text(mut self, content: impl Into<String>) -> Self {
        self.text = Some(content.into());
        self
    }

    pub fn text_color(mut self, color: impl IntoColor) -> Self {
        let rgb = color.into_color();
        self.text_color = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size.max(1.0);
        self
    }

    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&mut UIElement) + Send + Sync + 'static,
    {
        self.on_click = Some(Box::new(callback));
        self
    }

    /// Computes the absolute bounding box [min_x, min_y, max_x, max_y] in screen pixels.
    pub fn compute_bounds(&self, screen_w: f32, screen_h: f32) -> [f32; 4] {
        let origin = self.anchor.compute_origin(screen_w, screen_h);
        let mut min_x = origin.x + self.offset.x;
        let mut min_y = origin.y + self.offset.y;

        // Adjust position so anchor reflects element edges appropriately
        match self.anchor {
            Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => {
                min_x -= self.size.x / 2.0;
            }
            Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => {
                min_x -= self.size.x;
            }
            _ => {}
        }

        match self.anchor {
            Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => {
                min_y -= self.size.y / 2.0;
            }
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
                min_y -= self.size.y;
            }
            _ => {}
        }

        [min_x, min_y, min_x + self.size.x, min_y + self.size.y]
    }

    /// Checks if a screen mouse coordinate is inside this element.
    pub fn hit_test(&self, mouse_x: f32, mouse_y: f32, screen_w: f32, screen_h: f32) -> bool {
        let [min_x, min_y, max_x, max_y] = self.compute_bounds(screen_w, screen_h);
        mouse_x >= min_x && mouse_x <= max_x && mouse_y >= min_y && mouse_y <= max_y
    }

    /// Generates a 2D mesh representing this element in normalized screen coordinates.
    pub fn generate_mesh(&self, screen_w: f32, screen_h: f32) -> Mesh {
        self.generate_mesh_with_font(screen_w, screen_h, None)
    }

    /// Generates a 2D mesh representing this element with text glyphs rendered from FontAtlas.
    pub fn generate_mesh_with_font(&self, screen_w: f32, screen_h: f32, font: Option<&FontAtlas>) -> Mesh {
        let [min_x, min_y, max_x, max_y] = self.compute_bounds(screen_w, screen_h);

        // Convert screen pixel coordinates to orthographic coordinate space centered at (0,0)
        let half_w = screen_w / 2.0;
        let half_h = screen_h / 2.0;

        let left = min_x - half_w;
        let right = max_x - half_w;
        let top = half_h - min_y;
        let bottom = half_h - max_y;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let normal = [0.0, 0.0, 1.0];
        // Solid non-text quads use negative UV to bypass font texture sampling in WGSL shader
        let solid_uv = [-1.0, -1.0];

        // 1. Render Border (if configured)
        if self.border_width > 0.0 {
            let bc = [self.border_color.x, self.border_color.y, self.border_color.z, self.border_color.w];
            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [left,  bottom, 0.0], normal, uv: solid_uv, color: bc });
            vertices.push(Vertex { position: [right, bottom, 0.0], normal, uv: solid_uv, color: bc });
            vertices.push(Vertex { position: [right, top,    0.0], normal, uv: solid_uv, color: bc });
            vertices.push(Vertex { position: [left,  top,    0.0], normal, uv: solid_uv, color: bc });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // 2. Render Main Body Quad
        let b = self.border_width;
        let inner_left = left + b;
        let inner_right = right - b;
        let inner_bottom = bottom + b;
        let inner_top = top - b;

        let active_color = if self.is_hovered && self.hover_color.is_some() {
            self.hover_color.unwrap()
        } else {
            self.color
        };
        let c = [active_color.x, active_color.y, active_color.z, active_color.w];

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [inner_left,  inner_bottom, 0.0], normal, uv: solid_uv, color: c });
        vertices.push(Vertex { position: [inner_right, inner_bottom, 0.0], normal, uv: solid_uv, color: c });
        vertices.push(Vertex { position: [inner_right, inner_top,    0.0], normal, uv: solid_uv, color: c });
        vertices.push(Vertex { position: [inner_left,  inner_top,    0.0], normal, uv: solid_uv, color: c });
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

        // 3. Render Text Glyphs (if text and FontAtlas are available)
        if let (Some(text), Some(f)) = (&self.text, font) {
            let measured = f.measure_text(text, self.text_size);
            // Center horizontally within the element
            let origin_x = ((inner_left + inner_right) / 2.0) - (measured.x / 2.0);
            // Center vertically within the element
            let baseline_y = ((inner_bottom + inner_top) / 2.0) - (measured.y * 0.35);

            let (text_verts, text_indices) = f.layout_text(
                text,
                origin_x,
                baseline_y,
                self.text_size,
                self.text_color,
                vertices.len() as u32,
            );

            vertices.extend(text_verts);
            indices.extend(text_indices);
        }

        Mesh { vertices, indices }
    }
}
