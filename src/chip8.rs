use rand::{rngs::ThreadRng, Rng};
///
/// Chip8 interpreter
///
use std::fmt::Display;

use crate::common::USize;

const MEMORY_SIZE: usize = 4 * 1024;
const REGISTERS_COUNT: usize = 16;
const STACK_SIZE: usize = 16;

pub const DISPLAY_SIZE: USize = USize {
    height: 32,
    width: 64,
};

#[derive(Debug)]
pub enum Error {
    RomTooBig(usize),
    UnknownInstruction(Instruction),
    StackOverflow,
    EmptyStack,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RomTooBig(size) => write!(f, "Rom of size {size} bytes is too big"),
            Self::UnknownInstruction(instr) => write!(f, "Unknown instruction: {instr}"),
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::EmptyStack => write!(f, "Pop on empty stack"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum State {
    Running,
    Paused,
    Terminated,
}

const FONT_SPRITES: [u8; 5 * 16] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];
const FONT_BASE_ADDRESS: usize = 0x50;
const STACK_BASE_ADDRESS: usize = 0x00;
const PROGRAM_BASE_ADDRESS: usize = 0x200;

#[derive(Debug)]
pub struct Instruction {
    header: u8,
    nnn: u16,
    nn: u8,
    n: u8,
    x: usize,
    y: usize,
}

impl Instruction {
    fn with_opcode(opcode: u16) -> Self {
        Self {
            header: (opcode >> 12 & 0xf) as u8,
            nnn: opcode & 0xfff,
            nn: (opcode & 0xff) as u8,
            n: (opcode & 0x0f) as u8,
            x: (opcode >> 8 & 0xf) as usize,
            y: (opcode >> 4 & 0xf) as usize,
        }
    }

    fn with_bytes(high: u8, low: u8) -> Self {
        Self::with_opcode((high as u16) << 8 | low as u16)
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Header: {:x}, NNN: {:x}, NN: {:x}, N: {:x}, X: {:x}, Y:{:x}",
            self.header, self.nnn, self.nn, self.n, self.x, self.y
        )
    }
}
pub struct Chip8 {
    reg: [u8; REGISTERS_COUNT],
    reg_ptr: u16, // register pointer to memory
    timer_delay: u8,
    timer_sound: u8,
    sp: usize, // stack pointer
    pc: usize, // program counter
    memory: [u8; MEMORY_SIZE],
    video_memory: Vec<u8>,
    key_pressed: Option<u8>,
    state: State,
    rng: ThreadRng,
}

impl Chip8 {
    pub fn with_rom(rom: &[u8]) -> Result<Self, Error> {
        let mut memory = [0u8; MEMORY_SIZE];
        if rom.len() > MEMORY_SIZE - PROGRAM_BASE_ADDRESS {
            return Err(Error::RomTooBig(rom.len()));
        }
        for (i, val) in rom.iter().enumerate() {
            memory[PROGRAM_BASE_ADDRESS + i] = *val
        }
        for (i, val) in FONT_SPRITES.iter().enumerate() {
            memory[FONT_BASE_ADDRESS + i] = *val;
        }
        Ok(Self {
            reg: [0u8; REGISTERS_COUNT],
            reg_ptr: 0,
            timer_delay: 0,
            timer_sound: 0,
            sp: 0,
            pc: PROGRAM_BASE_ADDRESS,
            memory,
            video_memory: vec![0u8; DISPLAY_SIZE.square()],
            key_pressed: None,
            state: State::Running,
            rng: rand::thread_rng(),
        })
    }

    pub fn get_state(&self) -> State {
        self.state
    }

    pub fn terminate(&mut self) {
        self.state = State::Terminated
    }

    pub fn toggle_execution(&mut self) {
        self.state = match self.state {
            State::Paused => State::Running,
            State::Running => State::Paused,
            State::Terminated => State::Terminated,
        }
    }

    pub fn on_timer(&mut self) {
        self.timer_delay = self.timer_delay.saturating_sub(1);
        self.timer_sound = self.timer_sound.saturating_sub(1);
    }

    pub fn teak(&mut self) -> Result<(), Error> {
        let instr = Instruction::with_bytes(self.memory[self.pc], self.memory[self.pc + 1]);
        self.pc += 2;
        let (nnn, nn, n, x, y) = (instr.nnn, instr.nn, instr.n, instr.x, instr.y);
        match instr.header {
            0x0 => match nnn {
                0xe0 => self.op_clear_screen(),
                0xee => self.op_return()?,
                _ => {
                    // ignore machine code routine calls
                }
            },
            0x1 => self.op_jmp(nnn),
            0x2 => self.op_call(nnn)?,
            0x3 => self.op_skip_eq(x, nn),
            0x4 => self.op_skip_ne(x, nn),
            0x5 => self.op_skip_reg_eq(x, y),
            0x6 => self.op_mov(x, nn),
            0x7 => self.op_add(x, nn),
            0x8 => match n {
                0x0 => self.op_reg_mov(x, y),
                0x1 => self.op_or(x, y),
                0x2 => self.op_and(x, y),
                0x3 => self.op_xor(x, y),
                0x4 => self.op_reg_add(x, y),
                0x5 => self.op_reg_sub(x, y),
                0x6 => self.op_shr(x), // y should be ignored?
                0x7 => self.op_reg_sub_rev(x, y),
                0xe => self.op_shl(x), // y should be ignored?
                _ => {
                    return Err(Error::UnknownInstruction(instr));
                }
            },
            0x9 => self.op_skip_reg_ne(x, y),
            0xa => self.op_mov_ptr(nnn),
            0xb => self.op_reg0_jmp(nnn),
            0xc => self.op_rand(x, nn),
            0xd => self.op_display(x, y, n),
            0xe => match nn {
                0x9e => self.op_skip_key_eq(x),
                0xa1 => self.op_skip_key_ne(x),
                _ => {
                    return Err(Error::UnknownInstruction(instr));
                }
            },
            0xf => match nn {
                0x07 => self.op_dump_delay(x),
                0x0a => self.op_wait_key(x),
                0x15 => self.op_set_delay(x),
                0x18 => self.op_set_sound(x),
                0x1e => self.op_ptr_add(x),
                0x29 => self.op_mov_font_addr(x),
                0x33 => self.op_bdc(x),
                0x55 => self.op_reg_dump(x),
                0x65 => self.op_reg_load(x),
                _ => {
                    return Err(Error::UnknownInstruction(instr));
                }
            },
            _ => {
                return Err(Error::UnknownInstruction(instr));
            }
        }
        Ok(())
    }

    fn push(&mut self, value: u16) -> Result<(), Error> {
        if self.sp == STACK_SIZE {
            return Err(Error::StackOverflow);
        }
        let high = (value >> 8) as u8;
        let low = (value & 0xff) as u8;
        self.memory[STACK_BASE_ADDRESS + self.sp * 2] = high;
        self.memory[STACK_BASE_ADDRESS + self.sp * 2 + 1] = low;
        self.sp += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<u16, Error> {
        if self.sp == 0 {
            return Err(Error::EmptyStack);
        }
        self.sp -= 1;
        let high = self.memory[STACK_BASE_ADDRESS + self.sp * 2] as u16;
        let low = self.memory[STACK_BASE_ADDRESS + self.sp * 2 + 1] as u16;
        Ok(high << 8 | low)
    }

    fn op_clear_screen(&mut self) {
        self.video_memory.iter_mut().for_each(|val| *val = 0);
    }

    fn op_return(&mut self) -> Result<(), Error> {
        self.pc = self.pop()? as usize;
        Ok(())
    }

    fn op_jmp(&mut self, address: u16) {
        self.pc = address as usize;
    }

    fn op_call(&mut self, address: u16) -> Result<(), Error> {
        let ret_address = self.pc;
        self.push(ret_address as u16)?;
        self.pc = address as usize;
        Ok(())
    }

    fn op_skip_eq(&mut self, x: usize, value: u8) {
        if self.reg[x] == value {
            self.pc += 2;
        }
    }

    fn op_skip_ne(&mut self, x: usize, value: u8) {
        if self.reg[x] != value {
            self.pc += 2;
        }
    }

    fn op_skip_reg_eq(&mut self, x: usize, y: usize) {
        if self.reg[x] == self.reg[y] {
            self.pc += 2;
        }
    }

    fn op_mov(&mut self, x: usize, value: u8) {
        self.reg[x] = value;
    }

    fn op_add(&mut self, x: usize, value: u8) {
        let sum = value as u16 + self.reg[x] as u16;
        self.reg[x] = (sum & 0xff) as u8;
    }

    fn op_reg_mov(&mut self, x: usize, y: usize) {
        self.reg[x] = self.reg[y];
    }

    fn op_or(&mut self, x: usize, y: usize) {
        self.reg[x] |= self.reg[y];
    }

    fn op_and(&mut self, x: usize, y: usize) {
        self.reg[x] &= self.reg[y];
    }

    fn op_xor(&mut self, x: usize, y: usize) {
        self.reg[x] ^= self.reg[y];
    }

    fn op_reg_add(&mut self, x: usize, y: usize) {
        let a = self.reg[x] as u16;
        let b = self.reg[y] as u16;
        let sum = a + b;
        self.reg[0xf] = (sum > 0xff) as u8;
        self.reg[x] = (sum & 0xff) as u8;
    }

    fn op_reg_sub(&mut self, x: usize, y: usize) {
        let a = self.reg[x];
        let b = self.reg[y];
        self.reg[0xf] = (b < a) as u8;
        self.reg[x] = (a.wrapping_sub(b) & 0xff) as u8;
    }

    fn op_shr(&mut self, x: usize) {
        self.reg[0xf] = self.reg[x] & 1;
        self.reg[x] >>= 1;
    }

    fn op_reg_sub_rev(&mut self, x: usize, y: usize) {
        let a = self.reg[x];
        let b = self.reg[y];
        self.reg[0xf] = (a < b) as u8;
        self.reg[x] = (b.wrapping_sub(a) & 0xff) as u8;
    }

    fn op_shl(&mut self, x: usize) {
        self.reg[0xf] = self.reg[x] >> 7;
        self.reg[x] <<= 1;
    }

    fn op_skip_reg_ne(&mut self, x: usize, y: usize) {
        if self.reg[x] != self.reg[y] {
            self.pc += 2;
        }
    }

    fn op_mov_ptr(&mut self, address: u16) {
        self.reg_ptr = address;
    }

    fn op_reg0_jmp(&mut self, address: u16) {
        self.pc = self.reg[0] as usize + address as usize;
    }

    fn op_rand(&mut self, x: usize, value: u8) {
        self.reg[x] = value & self.rng.gen::<u8>();
    }

    fn op_display(&mut self, x: usize, y: usize, height: u8) {
        let height = height as usize;
        let row = self.reg[y] as usize % DISPLAY_SIZE.height;
        let col = self.reg[x] as usize % DISPLAY_SIZE.width;
        let ptr = self.reg_ptr as usize;
        self.reg[0xf] = 0;
        for (i, val) in self.memory[ptr..ptr + height].iter().enumerate() {
            let r = row + i;
            if r >= DISPLAY_SIZE.height {
                break;
            }
            for j in 0..8 {
                let c = col + j;
                if c >= DISPLAY_SIZE.width {
                    break;
                }
                let idx = r * DISPLAY_SIZE.width + c;
                let prev = self.video_memory[idx];
                let pixel = (val >> (7 - j)) & 1;
                if prev & pixel > 0 {
                    self.reg[0xf] = 1;
                }
                self.video_memory[idx] ^= pixel;
            }
        }
    }

    fn op_bdc(&mut self, x: usize) {
        let mut val = self.reg[x];
        let ptr = self.reg_ptr as usize;
        self.memory[ptr + 2] = val % 10;
        val /= 10;
        self.memory[ptr + 1] = val % 10;
        val /= 10;
        self.memory[ptr] = val % 10;
    }

    fn op_reg_dump(&mut self, x: usize) {
        let ptr = self.reg_ptr as usize;
        for offset in 0..=x {
            self.memory[ptr + offset] = self.reg[offset];
        }
    }

    fn op_reg_load(&mut self, x: usize) {
        let ptr = self.reg_ptr as usize;
        for offset in 0..=x {
            self.reg[offset] = self.memory[ptr + offset];
        }
    }

    fn op_ptr_add(&mut self, x: usize) {
        let val = self.reg[x];
        self.reg_ptr += val as u16;
    }

    fn op_mov_font_addr(&mut self, x: usize) {
        let val = self.reg[x] as u16;
        self.reg_ptr = FONT_BASE_ADDRESS as u16 + val * 5;
    }

    fn op_set_delay(&mut self, x: usize) {
        self.timer_delay = self.reg[x];
    }

    fn op_set_sound(&mut self, x: usize) {
        self.timer_sound = self.reg[x];
    }

    fn op_dump_delay(&mut self, x: usize) {
        self.reg[x] = self.timer_delay;
    }

    fn op_skip_key_eq(&mut self, x: usize) {
        if Some(self.reg[x]) == self.key_pressed {
            self.pc += 2;
        }
    }

    fn op_skip_key_ne(&mut self, x: usize) {
        if Some(self.reg[x]) != self.key_pressed {
            self.pc += 2;
        }
    }

    fn op_wait_key(&mut self, x: usize) {
        let Some(key_code) = self.key_pressed else {
            self.pc -= 2;
            return;
        };
        self.reg[x] = key_code;
    }

    pub fn get_video_ram(&self) -> &[u8] {
        &self.video_memory
    }

    pub fn key_down(&mut self, key_code: u8) {
        self.key_pressed = Some(key_code);
    }

    pub fn key_up(&mut self) {
        self.key_pressed = None;
    }

    pub fn is_audio_playing(&self) -> bool {
        self.timer_sound > 0
    }
}
