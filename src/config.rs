use std::io::{self, Error, ErrorKind};
use std::path::Path;

use crate::chip8::Quirks;

#[derive(Default)]
pub struct Config {
    pub appearance: AppearanceConfig,
    pub quirks: Quirks,
}

impl Config {
    pub fn with_file<P: AsRef<Path>>(_path: P) -> io::Result<Self> {
        Err(Error::new(
            ErrorKind::Other,
            "Loading config wasn't implemented yet",
        ))
    }
}

pub struct AppearanceConfig {
    pub scale: usize,
    pub color_foreground: u32,
    pub color_background: u32,
    pub is_pixel_style: bool,
    pub operations_per_second: u64,
    pub sound_volume: f32,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            scale: 16,
            color_foreground: 0xff_ff_ff,
            color_background: 0x00_00_00,
            is_pixel_style: true,
            operations_per_second: 800,
            sound_volume: 0.1,
        }
    }
}
