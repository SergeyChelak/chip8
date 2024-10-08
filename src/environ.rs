extern crate sdl2;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioStatus};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::{AudioSubsystem, Sdl, VideoSubsystem};

use crate::chip8::{self, Chip8, State};
use crate::config::AppearanceConfig;

pub struct Environment<'a> {
    sdl_context: Sdl,
    video_subsystem: VideoSubsystem,
    audio_subsystem: AudioSubsystem,
    config: AppearanceConfig,
    machine: &'a mut Chip8,
    key_mapping: HashMap<Keycode, u8>,
}

impl<'a> Environment<'a> {
    pub fn new(appearance: AppearanceConfig, machine: &'a mut Chip8) -> Result<Self, String> {
        let key_mapping = HashMap::from([
            (Keycode::Num1, 0x1),
            (Keycode::Num2, 0x2),
            (Keycode::Num3, 0x3),
            (Keycode::Num4, 0xc),
            (Keycode::Q, 0x4),
            (Keycode::W, 0x5),
            (Keycode::E, 0x6),
            (Keycode::R, 0xd),
            (Keycode::A, 0x7),
            (Keycode::S, 0x8),
            (Keycode::D, 0x9),
            (Keycode::F, 0xe),
            (Keycode::Z, 0xa),
            (Keycode::X, 0x0),
            (Keycode::C, 0xb),
            (Keycode::V, 0xf),
        ]);
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let audio_subsystem = sdl_context.audio()?;
        Ok(Self {
            sdl_context,
            video_subsystem,
            audio_subsystem,
            config: appearance,
            machine,
            key_mapping,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let dim = chip8::DISPLAY_SIZE * self.config.scale;
        // video
        let window = self
            .video_subsystem
            .window("Chip8", dim.width as u32, dim.height as u32)
            .position_centered()
            .build()
            .map_err(|op| op.to_string())?;
        let mut canvas = window.into_canvas().build().map_err(|op| op.to_string())?;
        // audio
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1), // mono
            samples: None,     // default sample size
        };

        let audio_device = self
            .audio_subsystem
            .open_playback(None, &desired_spec, |spec| {
                // initialize the audio callback
                SquareWave {
                    phase_inc: 220.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: self.config.sound_volume,
                }
            })
            .map_err(|op| op.to_string())?;
        audio_device.pause();
        // events
        let mut event_pump = self.sdl_context.event_pump()?;
        let mut refresh_time = Instant::now();
        let exp_duration = Duration::from_micros(1_000_000 / self.config.operations_per_second);
        'emu_loop: loop {
            let cycle_start = Instant::now();
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'emu_loop,
                    Event::KeyDown { keycode, .. } => self.on_key_down(keycode),
                    Event::KeyUp { keycode, .. } => self.on_key_up(keycode),
                    _ => {}
                }
            }
            match self.machine.get_state() {
                State::Terminated => break,
                State::Running => {
                    if let Err(error) = self.machine.teak() {
                        println!("Machine error: {}", error);
                        self.machine.terminate();
                    }
                }
                State::Paused => audio_device.pause(),
            }
            if refresh_time.elapsed().as_millis() >= 1000 / 60 {
                match (self.machine.is_audio_playing(), audio_device.status()) {
                    (false, AudioStatus::Playing) => audio_device.pause(),
                    (true, AudioStatus::Paused) => audio_device.resume(),
                    _ => {}
                };
                self.draw_display(&mut canvas)?;
                canvas.present();
                self.machine.on_timer();
                refresh_time = Instant::now();
            }
            let cycle_duration = cycle_start.elapsed();
            let sleep_time = exp_duration.saturating_sub(cycle_duration);
            if !sleep_time.is_zero() {
                ::std::thread::sleep(sleep_time);
            }
        }
        Ok(())
    }

    fn on_key_down(&mut self, keycode: Option<Keycode>) {
        let Some(keycode) = keycode else {
            return;
        };
        if let Some(code) = self.key_mapping.get(&keycode) {
            self.machine.key_down(*code);
            return;
        }
        match keycode {
            Keycode::Escape => self.machine.terminate(),
            Keycode::F5 => self.machine.toggle_execution(),
            Keycode::F9 => self.machine.reset(),
            _ => {
                // unhandled keys
            }
        }
    }

    fn on_key_up(&mut self, keycode: Option<Keycode>) {
        let Some(keycode) = keycode else {
            return;
        };
        if let Some(key_code) = self.key_mapping.get(&keycode) {
            self.machine.key_up(*key_code);
        }
    }

    fn draw_display(&mut self, canvas: &mut WindowCanvas) -> Result<(), String> {
        let memory = self.machine.get_video_ram();
        let size = self.config.scale;
        let bg_color = Color::RGB(
            self.config.background_red,
            self.config.background_green,
            self.config.background_blue,
        );
        let fg_color = Color::RGB(
            self.config.foreground_red,
            self.config.foreground_green,
            self.config.foreground_blue,
        );
        for r in 0..chip8::DISPLAY_SIZE.height {
            for c in 0..chip8::DISPLAY_SIZE.width {
                let idx = r * chip8::DISPLAY_SIZE.width + c;
                let color = if memory[idx] > 0 { fg_color } else { bg_color };
                canvas.set_draw_color(color);
                let rect = Rect::new(
                    (c * size) as i32,
                    (r * size) as i32,
                    size as u32,
                    size as u32,
                );
                canvas.fill_rect(rect)?;
                if self.config.is_pixel_style {
                    canvas.set_draw_color(bg_color);
                    canvas.draw_rect(rect)?;
                }
            }
        }
        Ok(())
    }
}

// https://docs.rs/sdl2/latest/sdl2/audio/index.html
struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}
