///
extern crate sdl2;

use std::collections::HashMap;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::{Sdl, VideoSubsystem};

use crate::chip8::{self, Chip8, State};
use crate::config::{self, Config};

pub struct Renderer<'a> {
    sdl_context: Sdl,
    video_subsystem: VideoSubsystem,
    config: Config,
    machine: &'a mut Chip8,
    key_mapping: HashMap<Keycode, u8>,
}

impl<'a> Renderer<'a> {
    pub fn new(config: Config, machine: &'a mut Chip8) -> Result<Self, String> {
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
        Ok(Self {
            sdl_context,
            video_subsystem,
            config,
            machine,
            key_mapping,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let dim = chip8::DISPLAY_SIZE * self.config.scale;

        let window = self
            .video_subsystem
            .window("Chip8", dim.width as u32, dim.height as u32)
            .position_centered()
            .build()
            .map_err(|op| op.to_string())?;
        let mut canvas = window.into_canvas().build().map_err(|op| op.to_string())?;
        let mut event_pump = self.sdl_context.event_pump()?;
        let bg_color = Color::from(self.config.color_background);
        'emu_loop: loop {
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
                State::Running if !self.machine.is_delayed() => {
                    if let Err(error) = self.machine.teak() {
                        println!("Machine error: {:?}", error);
                        self.machine.terminate();
                    }
                }
                _ => {}
            }
            canvas.set_draw_color(bg_color);
            canvas.clear();
            self.draw_display(&mut canvas)?;
            canvas.present();

            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 1000));

            self.machine.on_timer();
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
            _ => {
                // unhandled keys
            }
        }
    }

    fn on_key_up(&mut self, keycode: Option<Keycode>) {
        let Some(keycode) = keycode else {
            return;
        };
        if self.key_mapping.get(&keycode).is_some() {
            self.machine.key_up();
        }
    }

    fn draw_display(&mut self, canvas: &mut WindowCanvas) -> Result<(), String> {
        let memory = self.machine.get_video_ram();
        let size = self.config.scale;
        let bg_color = Color::from(self.config.color_background);
        let fg_color = Color::from(self.config.color_foreground);
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
                // canvas.draw_rect(rect)?;
                canvas.fill_rect(rect)?;
            }
        }
        Ok(())
    }
}

impl From<config::Color> for Color {
    fn from(value: config::Color) -> Self {
        Color::RGB(value.red, value.green, value.blue)
    }
}
