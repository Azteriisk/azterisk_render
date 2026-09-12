use crate::entity::{Mesh, Vertex};
use glam::{Vec2, Vec4};

/// Mouse cursor behavior and capture modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorMode {
    /// Normal operating system cursor with unconstrained movement.
    #[default]
    Normal,

    /// OS cursor is hidden, but moves freely within and outside window.
    Hidden,

    /// Mouse is captured/locked to window center (for 3D FPS, flight, or free-look).
    /// Raw mouse motion events control rotation without cursor boundary stops.
    Captured,

    /// Cursor is confined within the window boundaries.
    Confined,
}

/// In-game replacement cursor rendered via the 2D orthographic UI pipeline.
#[derive(Debug, Clone)]
pub enum CustomCursor {
    /// No custom cursor (OS cursor or completely invisible).
    None,

    /// Tactical 4-bar crosshair reticle.
    Crosshair {
        size: f32,
        thickness: f32,
        gap: f32,
        color: Vec4,
        dot: bool,
    },

    /// Circular aiming dot with optional glowing outer rim.
    Dot {
        radius: f32,
        color: Vec4,
        ring: bool,
    },

    /// Triangular arrow / pointer cursor with outline.
    Pointer {
        size: f32,
        color: Vec4,
        outline_color: Vec4,
    },

    /// Circular targeting ring.
    Ring {
        inner_radius: f32,
        outer_radius: f32,
        color: Vec4,
    },

    /// Custom user-defined 2D mesh.
    Mesh {
        mesh: Mesh,
        scale: f32,
    },
}

impl Default for CustomCursor {
    fn default() -> Self {
        Self::None
    }
}

impl CustomCursor {
    /// Returns a clean tactical sci-fi crosshair (cyan-green).
    pub fn crosshair() -> Self {
        Self::Crosshair {
            size: 10.0,
            thickness: 2.0,
            gap: 4.0,
            color: Vec4::new(0.0, 1.0, 0.85, 0.95),
            dot: true,
        }
    }

    /// Returns a glowing white aiming dot reticle with outer rim.
    pub fn dot() -> Self {
        Self::Dot {
            radius: 3.5,
            color: Vec4::new(1.0, 1.0, 1.0, 0.95),
            ring: true,
        }
    }

    /// Returns a modern arrow pointer cursor with dark outline.
    pub fn pointer() -> Self {
        Self::Pointer {
            size: 18.0,
            color: Vec4::new(0.15, 0.90, 0.80, 1.0),
            outline_color: Vec4::new(0.05, 0.05, 0.10, 0.9),
        }
    }

    /// Returns a circular targeting ring.
    pub fn ring() -> Self {
        Self::Ring {
            inner_radius: 12.0,
            outer_radius: 14.5,
            color: Vec4::new(0.20, 0.85, 1.0, 0.85),
        }
    }

    /// Generates the vertex/index geometry centered at the given screen-space coordinates.
    pub fn generate_mesh(&self, pos: Vec2) -> Mesh {
        match self {
            CustomCursor::None => Mesh::default(),
            CustomCursor::Crosshair {
                size,
                thickness,
                gap,
                color,
                dot,
            } => {
                let half_th = thickness * 0.5;
                let c = [color.x, color.y, color.z, color.w];
                let mut vertices = Vec::new();
                let mut indices = Vec::new();

                let mut add_quad = |x0: f32, y0: f32, x1: f32, y1: f32| {
                    let base = vertices.len() as u32;
                    vertices.push(Vertex {
                        position: [x0, y0, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [x1, y0, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [x1, y1, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [x0, y1, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                };

                // Left bar
                add_quad(pos.x - gap - size, pos.y - half_th, pos.x - gap, pos.y + half_th);
                // Right bar
                add_quad(pos.x + gap, pos.y - half_th, pos.x + gap + size, pos.y + half_th);
                // Top bar
                add_quad(pos.x - half_th, pos.y + gap, pos.x + half_th, pos.y + gap + size);
                // Bottom bar
                add_quad(pos.x - half_th, pos.y - gap - size, pos.x + half_th, pos.y - gap);

                if *dot {
                    let d = half_th * 1.2;
                    add_quad(pos.x - d, pos.y - d, pos.x + d, pos.y + d);
                }

                Mesh { vertices, indices }
            }
            CustomCursor::Dot { radius, color, ring } => {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let c = [color.x, color.y, color.z, color.w];
                let segments = 16;

                // Center dot
                let base = vertices.len() as u32;
                vertices.push(Vertex {
                    position: [pos.x, pos.y, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [-1.0, -1.0],
                    color: c,
                });
                let seg_f = segments as f32;
                for i in 0..segments {
                    let th = (i as f32 / seg_f) * std::f32::consts::TAU;
                    vertices.push(Vertex {
                        position: [pos.x + th.cos() * radius, pos.y + th.sin() * radius, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    let curr = base + 1 + i as u32;
                    let next = if i + 1 == segments { base + 1 } else { curr + 1 };
                    indices.extend_from_slice(&[base, curr, next]);
                }

                // Outer ring
                if *ring {
                    let r_in = radius * 2.2;
                    let r_out = radius * 2.8;
                    let ring_col = [color.x, color.y, color.z, color.w * 0.5];
                    let ring_base = vertices.len() as u32;
                    for i in 0..segments {
                        let th = (i as f32 / seg_f) * std::f32::consts::TAU;
                        let cos = th.cos();
                        let sin = th.sin();
                        vertices.push(Vertex {
                            position: [pos.x + cos * r_in, pos.y + sin * r_in, 0.0],
                            normal: [0.0, 0.0, 1.0],
                            uv: [-1.0, -1.0],
                            color: ring_col,
                        });
                        vertices.push(Vertex {
                            position: [pos.x + cos * r_out, pos.y + sin * r_out, 0.0],
                            normal: [0.0, 0.0, 1.0],
                            uv: [-1.0, -1.0],
                            color: ring_col,
                        });

                        let i0 = ring_base + (i * 2) as u32;
                        let i1 = i0 + 1;
                        let i2 = if i + 1 == segments { ring_base } else { i0 + 2 };
                        let i3 = if i + 1 == segments { ring_base + 1 } else { i0 + 3 };
                        indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
                    }
                }

                Mesh { vertices, indices }
            }
            CustomCursor::Pointer {
                size,
                color,
                outline_color,
            } => {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();

                let out_c = [outline_color.x, outline_color.y, outline_color.z, outline_color.w];
                let in_c = [color.x, color.y, color.z, color.w];

                // Outer border arrow
                let s_out = *size;
                let b_tip = [pos.x, pos.y, 0.0];
                let b_bl = [pos.x, pos.y - s_out, 0.0];
                let b_br = [pos.x + s_out * 0.72, pos.y - s_out * 0.72, 0.0];
                let b_notch = [pos.x + s_out * 0.28, pos.y - s_out * 0.65, 0.0];

                let base = vertices.len() as u32;
                vertices.push(Vertex { position: b_tip, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: out_c });
                vertices.push(Vertex { position: b_bl, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: out_c });
                vertices.push(Vertex { position: b_notch, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: out_c });
                vertices.push(Vertex { position: b_br, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: out_c });
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

                // Inner fill arrow (inset by 2.5px)
                let s_in = size - 3.5;
                if s_in > 4.0 {
                    let offset_y = -1.5;
                    let offset_x = 1.0;
                    let i_tip = [pos.x + offset_x, pos.y + offset_y, 0.0];
                    let i_bl = [pos.x + offset_x, pos.y + offset_y - s_in, 0.0];
                    let i_br = [pos.x + offset_x + s_in * 0.70, pos.y + offset_y - s_in * 0.70, 0.0];
                    let i_notch = [pos.x + offset_x + s_in * 0.28, pos.y + offset_y - s_in * 0.65, 0.0];

                    let in_base = vertices.len() as u32;
                    vertices.push(Vertex { position: i_tip, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: in_c });
                    vertices.push(Vertex { position: i_bl, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: in_c });
                    vertices.push(Vertex { position: i_notch, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: in_c });
                    vertices.push(Vertex { position: i_br, normal: [0.0, 0.0, 1.0], uv: [-1.0, -1.0], color: in_c });
                    indices.extend_from_slice(&[in_base, in_base + 1, in_base + 2, in_base, in_base + 2, in_base + 3]);
                }

                Mesh { vertices, indices }
            }
            CustomCursor::Ring {
                inner_radius,
                outer_radius,
                color,
            } => {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let c = [color.x, color.y, color.z, color.w];
                let segments = 24;
                let seg_f = segments as f32;

                for i in 0..segments {
                    let th = (i as f32 / seg_f) * std::f32::consts::TAU;
                    let cos = th.cos();
                    let sin = th.sin();
                    vertices.push(Vertex {
                        position: [pos.x + cos * inner_radius, pos.y + sin * inner_radius, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });
                    vertices.push(Vertex {
                        position: [pos.x + cos * outer_radius, pos.y + sin * outer_radius, 0.0],
                        normal: [0.0, 0.0, 1.0],
                        uv: [-1.0, -1.0],
                        color: c,
                    });

                    let i0 = (i * 2) as u32;
                    let i1 = i0 + 1;
                    let i2 = if i + 1 == segments { 0 } else { i0 + 2 };
                    let i3 = if i + 1 == segments { 1 } else { i0 + 3 };
                    indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
                }

                Mesh { vertices, indices }
            }
            CustomCursor::Mesh { mesh, scale } => {
                let mut vertices = mesh.vertices.clone();
                for v in &mut vertices {
                    v.position[0] = v.position[0] * scale + pos.x;
                    v.position[1] = v.position[1] * scale + pos.y;
                    v.uv = [-1.0, -1.0];
                }
                Mesh {
                    vertices,
                    indices: mesh.indices.clone(),
                }
            }
        }
    }
}
