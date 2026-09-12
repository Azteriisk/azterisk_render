use crate::entity::{Mesh, Vertex};
use glam::{IVec3, Vec3, Vec4};

/// Trait defining a voxel meshing strategy that converts a 3D volume into GPU geometry.
pub trait VoxelMesher<T> {
    fn mesh(&self, volume: &VoxelVolume<T>) -> Mesh;
}

/// Generic, cache-friendly 3D voxel volume container.
/// Coordinates span [-subdivisions ..= subdivisions] along X, Y, and Z.
#[derive(Clone)]
pub struct VoxelVolume<T> {
    pub subdivisions: i32,
    pub diameter: usize,
    data: Vec<T>,
    default_val: T,
}

impl<T: Copy + PartialEq + Default> VoxelVolume<T> {
    pub fn new(subdivisions: i32, default_val: T) -> Self {
        let subs = subdivisions.max(1);
        let diameter = (2 * subs + 1) as usize;
        let total = diameter * diameter * diameter;
        Self {
            subdivisions: subs,
            diameter,
            data: vec![default_val; total],
            default_val,
        }
    }

    #[inline]
    fn to_index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let s = self.subdivisions;
        if x < -s || x > s || y < -s || y > s || z < -s || z > s {
            return None;
        }
        let xi = (x + s) as usize;
        let yi = (y + s) as usize;
        let zi = (z + s) as usize;
        let d = self.diameter;
        Some((xi * d + yi) * d + zi)
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&T> {
        self.to_index(x, y, z).map(|idx| &self.data[idx])
    }

    #[inline]
    pub fn get_or_default(&self, x: i32, y: i32, z: i32) -> T {
        self.get(x, y, z).copied().unwrap_or(self.default_val)
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, val: T) -> bool {
        if let Some(idx) = self.to_index(x, y, z) {
            self.data[idx] = val;
            true
        } else {
            false
        }
    }

    /// Procedurally populates the volume using a closure `f(x, y, z) -> T`.
    pub fn fill<F>(&mut self, mut generator: F)
    where
        F: FnMut(i32, i32, i32) -> T,
    {
        let s = self.subdivisions;
        let d = self.diameter;
        for xi in 0..d {
            let x = xi as i32 - s;
            for yi in 0..d {
                let y = yi as i32 - s;
                for zi in 0..d {
                    let z = zi as i32 - s;
                    let idx = (xi * d + yi) * d + zi;
                    self.data[idx] = generator(x, y, z);
                }
            }
        }
    }

    /// Carves a spherical cavity in the voxel volume (e.g. for explosions or digging).
    pub fn carve_sphere(&mut self, center: IVec3, radius: f32, empty_val: T) {
        let r2 = radius * radius;
        let r_ceil = radius.ceil() as i32;
        let s = self.subdivisions;

        let min_x = (center.x - r_ceil).max(-s);
        let max_x = (center.x + r_ceil).min(s);
        let min_y = (center.y - r_ceil).max(-s);
        let max_y = (center.y + r_ceil).min(s);
        let min_z = (center.z - r_ceil).max(-s);
        let max_z = (center.z + r_ceil).min(s);

        for x in min_x..=max_x {
            let dx = (x - center.x) as f32;
            for y in min_y..=max_y {
                let dy = (y - center.y) as f32;
                for z in min_z..=max_z {
                    let dz = (z - center.z) as f32;
                    if dx * dx + dy * dy + dz * dz <= r2 {
                        self.set(x, y, z, empty_val);
                    }
                }
            }
        }
    }
}

/// Blisteringly fast face-culling mesher that inspects adjacent solid voxels.
/// Eliminates all internal occluded geometry, outputting a single unified [`Mesh`].
#[derive(Clone)]
pub struct CulledBlockMesher<T, S, C>
where
    S: Fn(&T) -> bool,
    C: Fn(&T, IVec3) -> Vec4,
{
    is_solid_fn: S,
    color_fn: C,
    pub enable_ao: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S, C> CulledBlockMesher<T, S, C>
where
    T: Copy + PartialEq + Default,
    S: Fn(&T) -> bool,
    C: Fn(&T, IVec3) -> Vec4,
{
    pub fn new(is_solid_fn: S, color_fn: C) -> Self {
        Self {
            is_solid_fn,
            color_fn,
            enable_ao: true,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Enables or disables smooth corner vertex ambient occlusion.
    pub fn ambient_occlusion(mut self, enabled: bool) -> Self {
        self.enable_ao = enabled;
        self
    }
}

impl<T, S, C> VoxelMesher<T> for CulledBlockMesher<T, S, C>
where
    T: Copy + PartialEq + Default,
    S: Fn(&T) -> bool,
    C: Fn(&T, IVec3) -> Vec4,
{
    fn mesh(&self, volume: &VoxelVolume<T>) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let s = volume.subdivisions;
        let is_solid = &self.is_solid_fn;
        let get_color = &self.color_fn;

        // 6 Cardinal Face Directions (Normal, [v0, v1, v2, v3], [(side1, side2, corner); 4])
        let faces: [(IVec3, Vec3, [[f32; 3]; 4], [(IVec3, IVec3, IVec3); 4]); 6] = [
            // +Y Top Face
            (
                IVec3::new(0, 1, 0),
                Vec3::new(0.0, 1.0, 0.0),
                [[-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
                [
                    (IVec3::new(-1, 1, 0), IVec3::new(0, 1, -1), IVec3::new(-1, 1, -1)),
                    (IVec3::new(1, 1, 0),  IVec3::new(0, 1, -1), IVec3::new(1, 1, -1)),
                    (IVec3::new(1, 1, 0),  IVec3::new(0, 1, 1),  IVec3::new(1, 1, 1)),
                    (IVec3::new(-1, 1, 0), IVec3::new(0, 1, 1),  IVec3::new(-1, 1, 1)),
                ],
            ),
            // -Y Bottom Face
            (
                IVec3::new(0, -1, 0),
                Vec3::new(0.0, -1.0, 0.0),
                [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5]],
                [
                    (IVec3::new(-1, -1, 0), IVec3::new(0, -1, 1),  IVec3::new(-1, -1, 1)),
                    (IVec3::new(1, -1, 0),  IVec3::new(0, -1, 1),  IVec3::new(1, -1, 1)),
                    (IVec3::new(1, -1, 0),  IVec3::new(0, -1, -1), IVec3::new(1, -1, -1)),
                    (IVec3::new(-1, -1, 0), IVec3::new(0, -1, -1), IVec3::new(-1, -1, -1)),
                ],
            ),
            // +Z Front Face
            (
                IVec3::new(0, 0, 1),
                Vec3::new(0.0, 0.0, 1.0),
                [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
                [
                    (IVec3::new(-1, 0, 1), IVec3::new(0, -1, 1), IVec3::new(-1, -1, 1)),
                    (IVec3::new(1, 0, 1),  IVec3::new(0, -1, 1), IVec3::new(1, -1, 1)),
                    (IVec3::new(1, 0, 1),  IVec3::new(0, 1, 1),  IVec3::new(1, 1, 1)),
                    (IVec3::new(-1, 0, 1), IVec3::new(0, 1, 1),  IVec3::new(-1, 1, 1)),
                ],
            ),
            // -Z Back Face
            (
                IVec3::new(0, 0, -1),
                Vec3::new(0.0, 0.0, -1.0),
                [[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]],
                [
                    (IVec3::new(1, 0, -1),  IVec3::new(0, -1, -1), IVec3::new(1, -1, -1)),
                    (IVec3::new(-1, 0, -1), IVec3::new(0, -1, -1), IVec3::new(-1, -1, -1)),
                    (IVec3::new(-1, 0, -1), IVec3::new(0, 1, -1),  IVec3::new(-1, 1, -1)),
                    (IVec3::new(1, 0, -1),  IVec3::new(0, 1, -1),  IVec3::new(1, 1, -1)),
                ],
            ),
            // +X Right Face
            (
                IVec3::new(1, 0, 0),
                Vec3::new(1.0, 0.0, 0.0),
                [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]],
                [
                    (IVec3::new(1, 0, 1),  IVec3::new(1, -1, 0), IVec3::new(1, -1, 1)),
                    (IVec3::new(1, 0, -1), IVec3::new(1, -1, 0), IVec3::new(1, -1, -1)),
                    (IVec3::new(1, 0, -1), IVec3::new(1, 1, 0),  IVec3::new(1, 1, -1)),
                    (IVec3::new(1, 0, 1),  IVec3::new(1, 1, 0),  IVec3::new(1, 1, 1)),
                ],
            ),
            // -X Left Face
            (
                IVec3::new(-1, 0, 0),
                Vec3::new(-1.0, 0.0, 0.0),
                [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]],
                [
                    (IVec3::new(-1, 0, -1), IVec3::new(-1, -1, 0), IVec3::new(-1, -1, -1)),
                    (IVec3::new(-1, 0, 1),  IVec3::new(-1, -1, 0), IVec3::new(-1, -1, 1)),
                    (IVec3::new(-1, 0, 1),  IVec3::new(-1, 1, 0),  IVec3::new(-1, 1, 1)),
                    (IVec3::new(-1, 0, -1), IVec3::new(-1, 1, 0),  IVec3::new(-1, 1, -1)),
                ],
            ),
        ];

        for x in -s..=s {
            for y in -s..=s {
                for z in -s..=s {
                    let voxel = volume.get_or_default(x, y, z);
                    if !is_solid(&voxel) {
                        continue;
                    }

                    let block_center = IVec3::new(x, y, z);
                    let col = get_color(&voxel, block_center);

                    let is_solid_at = |off: IVec3| -> bool {
                        let px = x + off.x;
                        let py = y + off.y;
                        let pz = z + off.z;
                        if px >= -s && px <= s && py >= -s && py <= s && pz >= -s && pz <= s {
                            is_solid(&volume.get_or_default(px, py, pz))
                        } else {
                            false
                        }
                    };

                    for (dir, normal_vec, corners, ao_corners) in &faces {
                        let nx = x + dir.x;
                        let ny = y + dir.y;
                        let nz = z + dir.z;

                        // Cull face if neighbor within chunk is solid
                        let neighbor_solid = if nx >= -s && nx <= s && ny >= -s && ny <= s && nz >= -s && nz <= s {
                            is_solid(&volume.get_or_default(nx, ny, nz))
                        } else {
                            false // Boundary faces are preserved
                        };

                        if !neighbor_solid {
                            let base = vertices.len() as u32;
                            let n = [normal_vec.x, normal_vec.y, normal_vec.z];

                            // Compute Ambient Occlusion for each corner
                            let mut ao_factors = [1.0f32; 4];
                            if self.enable_ao {
                                for (ci, (s1, s2, sc)) in ao_corners.iter().enumerate() {
                                    let side1 = is_solid_at(*s1);
                                    let side2 = is_solid_at(*s2);
                                    let corner = is_solid_at(*sc);

                                    let ao_level = if side1 && side2 {
                                        0
                                    } else {
                                        3 - (side1 as i32 + side2 as i32 + corner as i32)
                                    };

                                    ao_factors[ci] = match ao_level {
                                        3 => 1.0,
                                        2 => 0.80,
                                        1 => 0.62,
                                        _ => 0.44,
                                    };
                                }
                            }

                            for (ci, corner) in corners.iter().enumerate() {
                                let ao = ao_factors[ci];
                                vertices.push(Vertex {
                                    position: [
                                        x as f32 + corner[0],
                                        y as f32 + corner[1],
                                        z as f32 + corner[2],
                                    ],
                                    normal: n,
                                    uv: [0.0, 0.0],
                                    color: [col.x * ao, col.y * ao, col.z * ao, col.w],
                                });
                            }

                            // Anisotropy-aware quad triangulation to prevent lighting gradient artifacts
                            if ao_factors[0] + ao_factors[2] > ao_factors[1] + ao_factors[3] {
                                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                            } else {
                                indices.extend_from_slice(&[base + 1, base + 2, base + 3, base, base + 1, base + 3]);
                            }
                        }
                    }
                }
            }
        }

        Mesh { vertices, indices }
    }
}

/// Result of a successful 3D voxel raycast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelHit {
    /// The discrete coordinate of the impacted solid block.
    pub block_pos: IVec3,
    /// The face normal pointing out of the impacted block face.
    pub normal: IVec3,
    /// Distance from ray origin to impact.
    pub distance: f32,
}

/// Digital Differential Analyzer (3D DDA) raycaster traversing voxel grids in discrete steps.
pub fn raycast_voxel<T, S>(
    volume: &VoxelVolume<T>,
    ray_origin: Vec3,
    ray_dir: Vec3,
    max_distance: f32,
    is_solid: S,
) -> Option<VoxelHit>
where
    T: Copy + PartialEq + Default,
    S: Fn(&T) -> bool,
{
    let dir = ray_dir.normalize_or_zero();
    if dir.length_squared() < 0.001 {
        return None;
    }

    let mut curr = IVec3::new(
        ray_origin.x.round() as i32,
        ray_origin.y.round() as i32,
        ray_origin.z.round() as i32,
    );

    // Initial check at ray origin
    if let Some(val) = volume.get(curr.x, curr.y, curr.z) {
        if is_solid(val) {
            return Some(VoxelHit {
                block_pos: curr,
                normal: IVec3::ZERO,
                distance: 0.0,
            });
        }
    }

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::MAX };
    let t_delta_y = if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::MAX };
    let t_delta_z = if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::MAX };

    let mut t_max_x = if dir.x > 0.0 {
        (curr.x as f32 + 0.5 - ray_origin.x) * t_delta_x
    } else if dir.x < 0.0 {
        (ray_origin.x - (curr.x as f32 - 0.5)) * t_delta_x
    } else {
        f32::MAX
    };

    let mut t_max_y = if dir.y > 0.0 {
        (curr.y as f32 + 0.5 - ray_origin.y) * t_delta_y
    } else if dir.y < 0.0 {
        (ray_origin.y - (curr.y as f32 - 0.5)) * t_delta_y
    } else {
        f32::MAX
    };

    let mut t_max_z = if dir.z > 0.0 {
        (curr.z as f32 + 0.5 - ray_origin.z) * t_delta_z
    } else if dir.z < 0.0 {
        (ray_origin.z - (curr.z as f32 - 0.5)) * t_delta_z
    } else {
        f32::MAX
    };

    let mut dist = 0.0f32;
    let mut normal;

    while dist <= max_distance {
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                curr.x += step_x;
                dist = t_max_x;
                t_max_x += t_delta_x;
                normal = IVec3::new(-step_x, 0, 0);
            } else {
                curr.z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                normal = IVec3::new(0, 0, -step_z);
            }
        } else {
            if t_max_y < t_max_z {
                curr.y += step_y;
                dist = t_max_y;
                t_max_y += t_delta_y;
                normal = IVec3::new(0, -step_y, 0);
            } else {
                curr.z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                normal = IVec3::new(0, 0, -step_z);
            }
        }

        if let Some(val) = volume.get(curr.x, curr.y, curr.z) {
            if is_solid(val) {
                return Some(VoxelHit {
                    block_pos: curr,
                    normal,
                    distance: dist,
                });
            }
        }
    }

    None
}
