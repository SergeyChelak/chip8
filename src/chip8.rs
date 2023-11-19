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
const FONT_BASE_ADDRESS: usize = 0x050;
const STACK_BASE_ADDRESS: usize = 0x010;
const PROGRAM_BASE_ADDRESS: usize = 0x200;
const KB_WAIT_KEYCODE_ADDRESS: usize = 0x000;

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

pub struct Quirks {
    vf_reset: bool, // reset vf register after AND, OR, XOR operations
    memory: bool,   // increase RI after register dumb/load operations
    shifting: bool, // TRUE to SHR/SHL with Vx only, otherwise perform Vx = Vy before
    jumping: bool,
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

pub struct Chip8 {
    reg: [u8; REGISTERS_COUNT],
    ri: u16,   // indexing register
    dt: u8,    // delay timer
    st: u8,    // sound time
    sp: usize, // stack pointer
    pc: usize, // program counter
    memory: [u8; MEMORY_SIZE],
    video_memory: Vec<u8>,
    keypad: [bool; 0x10], // true if key pressed
    state: State,
    rng: ThreadRng,
    rom: Vec<u8>,
    quirks: Quirks,
}

impl Chip8 {
    pub fn with_rom(rom: Vec<u8>, quirks: Quirks) -> Result<Self, Error> {
        if rom.len() > MEMORY_SIZE - PROGRAM_BASE_ADDRESS {
            return Err(Error::RomTooBig(rom.len()));
        }
        let mut machine = Self {
            reg: [0u8; REGISTERS_COUNT],
            ri: 0,
            dt: 0,
            st: 0,
            sp: 0,
            pc: PROGRAM_BASE_ADDRESS,
            memory: [0u8; MEMORY_SIZE],
            video_memory: vec![0u8; DISPLAY_SIZE.square()],
            keypad: [false; 0x10],
            state: State::Paused,
            rng: rand::thread_rng(),
            rom,
            quirks,
        };
        machine.reset();
        Ok(machine)
    }

    pub fn reset(&mut self) {
        self.memory.iter_mut().for_each(|x| *x = 0);
        for (i, val) in self.rom.iter().enumerate() {
            self.memory[PROGRAM_BASE_ADDRESS + i] = *val
        }
        for (i, val) in FONT_SPRITES.iter().enumerate() {
            self.memory[FONT_BASE_ADDRESS + i] = *val;
        }
        self.reg.iter_mut().for_each(|x| *x = 0);
        self.ri = 0;
        self.dt = 0;
        self.st = 0;
        self.sp = 0;
        self.pc = PROGRAM_BASE_ADDRESS;
        self.video_memory.iter_mut().for_each(|x| *x = 0);
        self.keypad.iter_mut().for_each(|x| *x = false);
        self.state = State::Running;
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
        self.dt = self.dt.saturating_sub(1);
        self.st = self.st.saturating_sub(1);
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
                0x6 => self.op_shr(x, y),
                0x7 => self.op_reg_sub_rev(x, y),
                0xe => self.op_shl(x, y),
                _ => {
                    return Err(Error::UnknownInstruction(instr));
                }
            },
            0x9 => self.op_skip_reg_ne(x, y),
            0xa => self.op_mov_ptr(nnn),
            0xb => self.op_reg_jmp(nnn),
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
        if self.quirks.vf_reset {
            self.reg[0xf] = 0;
        }
    }

    fn op_and(&mut self, x: usize, y: usize) {
        self.reg[x] &= self.reg[y];
        if self.quirks.vf_reset {
            self.reg[0xf] = 0;
        }
    }

    fn op_xor(&mut self, x: usize, y: usize) {
        self.reg[x] ^= self.reg[y];
        if self.quirks.vf_reset {
            self.reg[0xf] = 0;
        }
    }

    fn op_reg_add(&mut self, x: usize, y: usize) {
        let a = self.reg[x] as u16;
        let b = self.reg[y] as u16;
        let sum = a + b;
        let vf = (sum > 0xff) as u8;
        self.reg[x] = (sum & 0xff) as u8;
        self.reg[0xf] = vf;
    }

    fn op_reg_sub(&mut self, x: usize, y: usize) {
        let a = self.reg[x];
        let b = self.reg[y];
        let vf = (b <= a) as u8;
        self.reg[x] = a.wrapping_sub(b);
        self.reg[0xf] = vf;
    }

    fn op_shr(&mut self, x: usize, y: usize) {
        if !self.quirks.shifting {
            self.reg[x] = self.reg[y];
        }
        let vf = self.reg[x] & 1;
        self.reg[x] >>= 1;
        self.reg[0xf] = vf;
    }

    fn op_reg_sub_rev(&mut self, x: usize, y: usize) {
        let a = self.reg[x];
        let b = self.reg[y];
        let vf = (a <= b) as u8;
        self.reg[x] = b.wrapping_sub(a);
        self.reg[0xf] = vf;
    }

    fn op_shl(&mut self, x: usize, y: usize) {
        if !self.quirks.shifting {
            self.reg[x] = self.reg[y];
        }
        let vf = self.reg[x] >> 7;
        self.reg[x] <<= 1;
        self.reg[0xf] = vf;
    }

    fn op_skip_reg_ne(&mut self, x: usize, y: usize) {
        if self.reg[x] != self.reg[y] {
            self.pc += 2;
        }
    }

    fn op_mov_ptr(&mut self, address: u16) {
        self.ri = address;
    }

    fn op_reg_jmp(&mut self, address: u16) {
        let base = if self.quirks.jumping {
            self.reg[((address >> 8) & 0xf) as usize]
        } else {
            self.reg[0]
        };
        self.pc = base as usize + address as usize;
    }

    fn op_rand(&mut self, x: usize, value: u8) {
        self.reg[x] = value & self.rng.gen::<u8>();
    }

    fn op_display(&mut self, x: usize, y: usize, height: u8) {
        let height = height as usize;
        let row = self.reg[y] as usize % DISPLAY_SIZE.height;
        let col = self.reg[x] as usize % DISPLAY_SIZE.width;
        let ptr = self.ri as usize;
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
        let val = self.reg[x];
        let ptr = self.ri as usize;
        self.memory[ptr] = val / 100 % 10;
        self.memory[ptr + 1] = val / 10 % 10;
        self.memory[ptr + 2] = val % 10;
    }

    fn op_reg_dump(&mut self, x: usize) {
        let ptr = self.ri as usize;
        for offset in 0..=x {
            self.memory[ptr + offset] = self.reg[offset];
        }
        if self.quirks.memory {
            self.ri += x as u16 + 1;
        }
    }

    fn op_reg_load(&mut self, x: usize) {
        let ptr = self.ri as usize;
        for offset in 0..=x {
            self.reg[offset] = self.memory[ptr + offset];
        }
        if self.quirks.memory {
            self.ri += x as u16 + 1;
        }
    }

    fn op_ptr_add(&mut self, x: usize) {
        let val = self.reg[x];
        self.ri += val as u16;
    }

    fn op_mov_font_addr(&mut self, x: usize) {
        let val = self.reg[x] as u16;
        self.ri = FONT_BASE_ADDRESS as u16 + val * 5;
    }

    fn op_set_delay(&mut self, x: usize) {
        self.dt = self.reg[x];
    }

    fn op_set_sound(&mut self, x: usize) {
        self.st = self.reg[x];
    }

    fn op_dump_delay(&mut self, x: usize) {
        self.reg[x] = self.dt;
    }

    fn op_skip_key_eq(&mut self, x: usize) {
        if self.keypad[self.reg[x] as usize] {
            self.pc += 2;
        }
    }

    fn op_skip_key_ne(&mut self, x: usize) {
        if !self.keypad[self.reg[x] as usize] {
            self.pc += 2;
        }
    }

    fn op_wait_key(&mut self, x: usize) {
        // Highest bit is a flag that displays if key already pressed
        // Lowest bits are representing keycode
        let val = self.memory[KB_WAIT_KEYCODE_ADDRESS];
        let is_pressed = val >> 7 == 1;
        if is_pressed {
            let key_code = val & 0xf;
            if !self.keypad[key_code as usize] {
                self.reg[x] = key_code;
                self.memory[KB_WAIT_KEYCODE_ADDRESS] = 0;
                return;
            }
        } else if let Some((key_code, _)) = self
            .keypad
            .iter()
            .enumerate()
            .find(|&(_, is_pressed)| *is_pressed)
        {
            self.memory[KB_WAIT_KEYCODE_ADDRESS] = 1 << 7 | key_code as u8;
        }
        self.pc -= 2;
    }

    pub fn get_video_ram(&self) -> &[u8] {
        &self.video_memory
    }

    pub fn key_down(&mut self, key_code: u8) {
        self.keypad[key_code as usize] = true;
    }

    pub fn key_up(&mut self, key_code: u8) {
        self.keypad[key_code as usize] = false;
    }

    pub fn is_audio_playing(&self) -> bool {
        self.st > 0
    }
}
