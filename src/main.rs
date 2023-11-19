use std::env;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

mod chip8;
use chip8::*;

mod config;
use config::Config;

mod common;

mod environ;
use environ::Environment;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        show_usage();
        return;
    }
    let config = Config::with_file("chip.cfg").unwrap_or_default();

    // setup chip8
    let Ok(rom) = load_rom(&args[1]) else {
        println!("Failed to load ROM {}", args[1]);
        return;
    };
    let Ok(mut machine) = Chip8::with_rom(rom, config.quirks) else {
        println!("Failed to load program into memory");
        return;
    };
    let mut environ =
        Environment::new(config.appearance, &mut machine).expect("Failed to setup SDL2");
    _ = environ.run();
}

fn show_usage() {
    println!("Here should be a manual of how to use interpreter");
}

fn load_rom<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
