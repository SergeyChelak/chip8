use std::fs;
use std::io;
use std::path::Path;

use serde_derive::Deserialize;

#[derive(Default, Deserialize)]
pub struct Config {
    pub appearance: AppearanceConfig,
    pub quirks: Quirks,
}

impl Config {
    pub fn with_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Deserialize)]
pub struct AppearanceConfig {
    pub scale: usize,
    pub foreground_red: u8,
    pub foreground_green: u8,
    pub foreground_blue: u8,
    pub background_red: u8,
    pub background_green: u8,
    pub background_blue: u8,
    pub is_pixel_style: bool,
    pub operations_per_second: u64,
    pub sound_volume: f32,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            scale: 16,
            foreground_red: 0xff,
            foreground_green: 0xff,
            foreground_blue: 0xff,
            background_red: 0,
            background_green: 0,
            background_blue: 0,
            is_pixel_style: true,
            operations_per_second: 800,
            sound_volume: 0.1,
        }
    }
}

#[derive(Deserialize)]
pub struct Quirks {
    pub vf_reset: bool, // reset vf register after AND, OR, XOR operations
    pub memory: bool,   // increase RI after register dumb/load operations
    pub shifting: bool, // TRUE to SHR/SHL with Vx only, otherwise perform Vx = Vy before
    pub jumping: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            vf_reset: true,
            memory: false,
            shifting: true,
            jumping: false,
        }
    }
}
