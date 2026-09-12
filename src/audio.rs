use crate::spatial::{grid_to_world_3d, ChunkCoord3D, IntoChunkCoord3D, IntoSubCoord3D, SubCoord3D};
use glam::Vec3;
use rodio::source::Source;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, SpatialPlayer};
use std::fs::File;

/// Represents an active 3D spatial sound source anchored in the world grid.
pub struct SpatialSound {
    pub path: String,
    pub chunk: ChunkCoord3D,
    pub sub: SubCoord3D,
    pub world_pos: Vec3,
    pub radius: f32,       // Audible distance in block units
    pub intensity: f32,    // Volume multiplier
    pub is_looping: bool,
    player: Option<SpatialPlayer>,
}

impl SpatialSound {
    pub fn new(
        path: impl Into<String>,
        chunk: ChunkCoord3D,
        sub: SubCoord3D,
        subdivisions: i32,
    ) -> Self {
        let world_pos = grid_to_world_3d(chunk, sub, subdivisions);
        Self {
            path: path.into(),
            chunk,
            sub,
            world_pos,
            radius: 30.0,    // Default: audible within 30 blocks
            intensity: 1.0,  // Default: 100% volume
            is_looping: false,
            player: None,
        }
    }

    /// Sets the audible radius in block units (e.g. 15.0 blocks for a torch, 60.0 for thunder).
    pub fn radius(&mut self, blocks: f32) -> &mut Self {
        self.radius = blocks.max(0.1);
        self
    }

    /// Sets the volume intensity (0.0 to 1.0+).
    pub fn intensity(&mut self, volume: f32) -> &mut Self {
        self.intensity = volume.max(0.0);
        if let Some(player) = &self.player {
            player.set_volume(self.intensity);
        }
        self
    }

    /// Enables or disables sound looping.
    pub fn looping(&mut self, is_looping: bool) -> &mut Self {
        self.is_looping = is_looping;
        self
    }
}

/// Central audio manager handling 2D background music, UI sound effects,
/// and 3D spatial audio anchored to the coordinate grid.
pub struct AudioManager {
    sink: Option<MixerDeviceSink>,
    music_player: Option<Player>,
    spatial_sounds: Vec<SpatialSound>,
}

impl AudioManager {
    pub fn new() -> Self {
        let sink = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[azterisk_render audio note] No audio output device available ({e}). Audio will be disabled.");
                None
            }
        };

        Self {
            sink,
            music_player: None,
            spatial_sounds: Vec::new(),
        }
    }

    /// Plays 2D background music with optional looping and volume.
    pub fn play_music(&mut self, path: impl Into<String>, looping: bool, volume: f32) {
        let sink = match &self.sink {
            Some(s) => s,
            None => return,
        };

        let path_str = path.into();
        let file = match File::open(&path_str) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[azterisk_render warning] Failed to open music file '{path_str}': {e}");
                return;
            }
        };

        let source = match Decoder::try_from(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[azterisk_render warning] Failed to decode music file '{path_str}': {e}");
                return;
            }
        };

        let player = Player::connect_new(&sink.mixer());
        player.set_volume(volume.max(0.0));
        if looping {
            player.append(source.repeat_infinite());
        } else {
            player.append(source);
        }

        self.music_player = Some(player);
    }

    /// Plays an instantaneous 2D sound effect (e.g. UI click, menu chime).
    pub fn play_sfx(&self, path: impl Into<String>, volume: f32) {
        let sink = match &self.sink {
            Some(s) => s,
            None => return,
        };

        let path_str = path.into();
        let file = match File::open(&path_str) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[azterisk_render warning] Failed to open SFX '{path_str}': {e}");
                return;
            }
        };

        let source = match Decoder::try_from(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[azterisk_render warning] Failed to decode SFX '{path_str}': {e}");
                return;
            }
        };

        let player = Player::connect_new(&sink.mixer());
        player.set_volume(volume.max(0.0));
        player.append(source);
        player.detach();
    }

    /// Adds a 3D spatial sound anchored to a grid cell.
    pub fn add_spatial_sound(
        &mut self,
        path: impl Into<String>,
        chunk_coord: impl IntoChunkCoord3D,
        sub_coord: impl IntoSubCoord3D,
        subdivisions: i32,
    ) -> &mut SpatialSound {
        let chunk = chunk_coord.into_chunk_coord().unwrap_or_default();
        let sub = sub_coord.into_sub_coord().unwrap_or_default();

        let sound = SpatialSound::new(path, chunk, sub, subdivisions);
        self.spatial_sounds.push(sound);
        self.spatial_sounds.last_mut().unwrap()
    }

    /// Updates listener position and ear orientation from active camera every frame.
    pub fn update_listener(&mut self, eye_pos: Vec3, _forward: Vec3, right: Vec3) {
        let sink = match &self.sink {
            Some(s) => s,
            None => return,
        };

        let ear_separation = 0.25f32; // Standard human ear separation in world units
        let left_ear = eye_pos - right * ear_separation;
        let right_ear = eye_pos + right * ear_separation;

        for sound in &mut self.spatial_sounds {
            if sound.player.is_none() {
                let file = match File::open(&sound.path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let source = match Decoder::try_from(file) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let player = SpatialPlayer::connect_new(
                    &sink.mixer(),
                    [sound.world_pos.x, sound.world_pos.y, sound.world_pos.z],
                    [left_ear.x, left_ear.y, left_ear.z],
                    [right_ear.x, right_ear.y, right_ear.z],
                );

                player.set_volume(sound.intensity);
                if sound.is_looping {
                    player.append(source.repeat_infinite());
                } else {
                    player.append(source);
                }

                sound.player = Some(player);
            }

            if let Some(player) = &mut sound.player {
                player.set_left_ear_position([left_ear.x, left_ear.y, left_ear.z]);
                player.set_right_ear_position([right_ear.x, right_ear.y, right_ear.z]);
                player.set_emitter_position([sound.world_pos.x, sound.world_pos.y, sound.world_pos.z]);

                // Distance attenuation relative to configured radius in block units
                let dist = (eye_pos - sound.world_pos).length();
                let attenuation = (1.0 - (dist / sound.radius)).clamp(0.0, 1.0);
                player.set_volume(sound.intensity * attenuation * attenuation);
            }
        }
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}
