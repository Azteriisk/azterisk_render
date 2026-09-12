use crate::errors::{AzteriskError, Result};
use glam::{Vec2, Vec3};
use std::fmt;
use std::str::FromStr;

/// 3D Chunk coordinate within [-world_span ..= world_span]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkCoord3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord3D {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn validate(&self, span: i32) -> Result<()> {
        if self.x.abs() > span || self.y.abs() > span || self.z.abs() > span {
            return Err(AzteriskError::ChunkOutOfBounds {
                x: self.x,
                y: self.y,
                z: self.z,
                span,
            });
        }
        Ok(())
    }
}

impl fmt::Display for ChunkCoord3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}x{}", self.x, self.y, self.z)
    }
}

impl FromStr for ChunkCoord3D {
    type Err = AzteriskError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('x').map(|p| p.trim()).collect();
        if parts.len() != 3 {
            return Err(AzteriskError::InvalidCoordinateString {
                raw: s.to_string(),
            });
        }
        let x = parts[0].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let y = parts[1].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let z = parts[2].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        Ok(Self::new(x, y, z))
    }
}

/// 3D Sub-cell (block/voxel) coordinate within [-subdivisions ..= subdivisions]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SubCoord3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl SubCoord3D {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn validate(&self, subdivisions: i32) -> Result<()> {
        if self.x.abs() > subdivisions || self.y.abs() > subdivisions || self.z.abs() > subdivisions {
            return Err(AzteriskError::SubCoordOutOfBounds {
                x: self.x,
                y: self.y,
                z: self.z,
                subdivisions,
            });
        }
        Ok(())
    }
}

impl fmt::Display for SubCoord3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.x, self.y, self.z)
    }
}

impl FromStr for SubCoord3D {
    type Err = AzteriskError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        if parts.len() != 3 {
            return Err(AzteriskError::InvalidCoordinateString {
                raw: s.to_string(),
            });
        }
        let x = parts[0].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let y = parts[1].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let z = parts[2].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        Ok(Self::new(x, y, z))
    }
}

/// 2D Chunk coordinate within [-world_span ..= world_span]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkCoord2D {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord2D {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn validate(&self, span: i32) -> Result<()> {
        if self.x.abs() > span || self.y.abs() > span {
            return Err(AzteriskError::ChunkOutOfBounds {
                x: self.x,
                y: self.y,
                z: 0,
                span,
            });
        }
        Ok(())
    }
}

impl FromStr for ChunkCoord2D {
    type Err = AzteriskError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('x').map(|p| p.trim()).collect();
        if parts.len() != 2 {
            return Err(AzteriskError::InvalidCoordinateString {
                raw: s.to_string(),
            });
        }
        let x = parts[0].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let y = parts[1].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        Ok(Self::new(x, y))
    }
}

/// 2D Sub-cell coordinate within [-subdivisions ..= subdivisions]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SubCoord2D {
    pub x: i32,
    pub y: i32,
}

impl SubCoord2D {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn validate(&self, subdivisions: i32) -> Result<()> {
        if self.x.abs() > subdivisions || self.y.abs() > subdivisions {
            return Err(AzteriskError::SubCoordOutOfBounds {
                x: self.x,
                y: self.y,
                z: 0,
                subdivisions,
            });
        }
        Ok(())
    }
}

impl FromStr for SubCoord2D {
    type Err = AzteriskError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        if parts.len() != 2 {
            return Err(AzteriskError::InvalidCoordinateString {
                raw: s.to_string(),
            });
        }
        let x = parts[0].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        let y = parts[1].parse().map_err(|_| AzteriskError::InvalidCoordinateString { raw: s.to_string() })?;
        Ok(Self::new(x, y))
    }
}

/// Helper traits to allow passing "&str", `(i32, i32, i32)`, or `ChunkCoord3D` seamlessly.
pub trait IntoChunkCoord3D {
    fn into_chunk_coord(self) -> Result<ChunkCoord3D>;
}

impl IntoChunkCoord3D for ChunkCoord3D {
    fn into_chunk_coord(self) -> Result<ChunkCoord3D> {
        Ok(self)
    }
}

impl IntoChunkCoord3D for &str {
    fn into_chunk_coord(self) -> Result<ChunkCoord3D> {
        self.parse()
    }
}

impl IntoChunkCoord3D for (i32, i32, i32) {
    fn into_chunk_coord(self) -> Result<ChunkCoord3D> {
        Ok(ChunkCoord3D::new(self.0, self.1, self.2))
    }
}

pub trait IntoSubCoord3D {
    fn into_sub_coord(self) -> Result<SubCoord3D>;
}

impl IntoSubCoord3D for SubCoord3D {
    fn into_sub_coord(self) -> Result<SubCoord3D> {
        Ok(self)
    }
}

impl IntoSubCoord3D for &str {
    fn into_sub_coord(self) -> Result<SubCoord3D> {
        self.parse()
    }
}

impl IntoSubCoord3D for (i32, i32, i32) {
    fn into_sub_coord(self) -> Result<SubCoord3D> {
        Ok(SubCoord3D::new(self.0, self.1, self.2))
    }
}

/// Calculates the continuous world space `Vec3` for a given chunk and sub-cell.
/// Each sub-cell is exactly 1.0 unit in size.
/// A chunk spans [-subdivisions ..= subdivisions], so its diameter is `2 * subdivisions` units.
pub fn grid_to_world_3d(chunk: ChunkCoord3D, sub: SubCoord3D, subdivisions: i32) -> Vec3 {
    let chunk_diameter = (subdivisions.max(1) * 2) as f32;
    let world_x = chunk.x as f32 * chunk_diameter + sub.x as f32;
    let world_y = chunk.y as f32 * chunk_diameter + sub.y as f32;
    let world_z = chunk.z as f32 * chunk_diameter + sub.z as f32;
    Vec3::new(world_x, world_y, world_z)
}

/// Calculates the continuous world space `Vec2` for 2D mode.
pub fn grid_to_world_2d(chunk: ChunkCoord2D, sub: SubCoord2D, subdivisions: i32) -> Vec2 {
    let chunk_diameter = (subdivisions.max(1) * 2) as f32;
    let world_x = chunk.x as f32 * chunk_diameter + sub.x as f32;
    let world_y = chunk.y as f32 * chunk_diameter + sub.y as f32;
    Vec2::new(world_x, world_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_parsing() {
        let coord: ChunkCoord3D = "0x0x1".parse().unwrap();
        assert_eq!(coord, ChunkCoord3D::new(0, 0, 1));

        let neg: ChunkCoord3D = "-5x10x-20".parse().unwrap();
        assert_eq!(neg, ChunkCoord3D::new(-5, 10, -20));
    }

    #[test]
    fn test_sub_coord_parsing() {
        let sub: SubCoord3D = "0,0,0".parse().unwrap();
        assert_eq!(sub, SubCoord3D::new(0, 0, 0));

        let sub2: SubCoord3D = "-32, 64, -12".parse().unwrap();
        assert_eq!(sub2, SubCoord3D::new(-32, 64, -12));
    }

    #[test]
    fn test_bounds_validation() {
        let chunk = ChunkCoord3D::new(101, 0, 0);
        assert!(chunk.validate(100).is_err());

        let sub = SubCoord3D::new(0, 65, 0);
        assert!(sub.validate(64).is_err());

        let valid_chunk = ChunkCoord3D::new(100, -100, 50);
        assert!(valid_chunk.validate(100).is_ok());

        let valid_sub = SubCoord3D::new(64, -64, 0);
        assert!(valid_sub.validate(64).is_ok());
    }

    #[test]
    fn test_world_conversion() {
        // Center chunk (0,0,0) and center sub (0,0,0) -> (0,0,0)
        let w0 = grid_to_world_3d(ChunkCoord3D::new(0, 0, 0), SubCoord3D::new(0, 0, 0), 64);
        assert_eq!(w0, Vec3::ZERO);

        // Chunk (0, 0, 1) with sub (0, 0, 0) and subdivisions 64 -> (0, 0, 128)
        let w1 = grid_to_world_3d(ChunkCoord3D::new(0, 0, 1), SubCoord3D::new(0, 0, 0), 64);
        assert_eq!(w1, Vec3::new(0.0, 0.0, 128.0));
    }
}
