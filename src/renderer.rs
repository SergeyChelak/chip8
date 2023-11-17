///
extern crate sdl2;

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
}

impl<'a> Renderer<'a> {
    pub fn new(config: Config, machine: &'a mut Chip8) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        Ok(Self {
            sdl_context,
            video_subsystem,
            config,
            machine,
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
                    Event::KeyDown { keycode, .. } => self.handle_keydown(keycode),
                    _ => {}
                }
            }
            match self.machine.get_state() {
                State::Terminated => break,
                State::Running => {
                    self.machine.teak();
                }
                _ => {}
            }
            canvas.set_draw_color(bg_color);
            canvas.clear();
            self.draw_display(&mut canvas)?;
            canvas.present();

            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 40));

            self.machine.on_timer();
        }
        Ok(())
    }

    fn handle_keydown(&mut self, keycode: Option<Keycode>) {
        let Some(keycode) = keycode else {
            return;
        };
        match keycode {
            Keycode::Escape => self.machine.terminate(),
            Keycode::F5 => self.machine.toggle_execution(),
            _ => {
                // unhandled
            }
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
