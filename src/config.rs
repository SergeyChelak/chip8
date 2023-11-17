use std::io::{self, Error, ErrorKind};
use std::path::Path;

#[derive(Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

pub struct Config {
    pub scale: usize,
    pub color_foreground: Color,
    pub color_background: Color,
}

impl Config {
    pub fn with_file<P: AsRef<Path>>(_path: P) -> io::Result<Self> {
        Err(Error::new(
            ErrorKind::Other,
            "Loading config wasn't implemented yet",
        ))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scale: 25,
            color_foreground: Color {
                red: 0xff,
                green: 0xff,
                blue: 0xff,
            },
            color_background: Color {
                red: 0x00,
                green: 0x00,
                blue: 0x00,
            },
        }
    }
}
