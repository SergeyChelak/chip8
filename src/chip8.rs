///
/// Chip8 interpreter
///
use rand::{rngs::ThreadRng, Rng};

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
    MemoryFault(usize),          // access to wrong address
    CallMachineCodeRoutine(u16), // call machine code at address
    EmptyOpcode,
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
const FONT_START_ADDRESS: usize = 0x50;

pub struct Chip8 {
    reg: [u8; REGISTERS_COUNT],
    reg_ptr: u16, // register pointer to memory
    timer_delay: u8,
    timer_sound: u8,
    sp: usize, // stack pointer
    stack: [usize; STACK_SIZE],
    pc: usize, // program counter
    memory: [u8; MEMORY_SIZE],
    video_memory: Vec<u8>,
    key_pressed: Option<u8>,
    state: State,
    rng: ThreadRng,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut memory = [0u8; MEMORY_SIZE];
        for (i, val) in FONT_SPRITES.iter().enumerate() {
            memory[FONT_START_ADDRESS + i] = *val;
        }
        Self {
            reg: [0u8; REGISTERS_COUNT],
            reg_ptr: 0,
            timer_delay: 0,
            timer_sound: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            pc: 0,
            memory,
            video_memory: vec![0u8; DISPLAY_SIZE.square()],
            key_pressed: None,
            state: State::Running,
            rng: rand::thread_rng(),
        }
    }

    pub fn load_rom(&mut self, program: Vec<u8>) -> Result<usize, Error> {
        let offset = 0x200;
        for (i, val) in program.iter().enumerate() {
            let address = offset + i;
            if address >= MEMORY_SIZE {
                return Err(Error::MemoryFault(address));
            }
            self.memory[address] = *val
        }
        self.pc = offset;
        Ok(program.len())
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
        let opcode = (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16;
        if opcode == 0 {
            return Err(Error::EmptyOpcode);
        }
        self.pc += 2;

        // decode
        // NNN: address
        // NN: 8-bit constant
        // N: 4-bit constant
        // X and Y: 4-bit register identifier
        // PC : Program Counter
        // I : 16bit register (For memory address) (Similar to void pointer);
        // VN: One of the 16 available variables. N may be 0 to F (hexadecimal);

        // P****
        let prefix = opcode >> 12;

        // *nnn
        let address = opcode & 0xfff;

        // **nn
        let constant = (opcode & 0xff) as u8;

        // *X**
        let reg_x = opcode >> 8 & 0xf;
        // **Y*
        let reg_y = opcode >> 4 & 0xf;

        // ***S
        let suffix = (opcode & 0x0f) as u8;
        match prefix {
            0x0 => match address {
                0xe0 => self.op_clear_screen(),
                0xee => self.op_return(),
                _ => return Err(Error::CallMachineCodeRoutine(address)),
            },
            0x1 => self.op_jmp(address),
            0x2 => self.op_call(address),
            0x3 => self.op_skip_eq(reg_x, constant),
            0x4 => self.op_skip_ne(reg_x, constant),
            0x5 => {
                assert_eq!(suffix, 0);
                self.op_skip_reg_eq(reg_x, reg_y)
            }
            0x6 => self.op_mov(reg_x, constant),
            0x7 => self.op_add(reg_x, constant),
            0x8 => match suffix {
                0x0 => self.op_reg_mov(reg_x, reg_y),
                0x1 => self.op_or(reg_x, reg_y),
                0x2 => self.op_and(reg_x, reg_y),
                0x3 => self.op_xor(reg_x, reg_y),
                0x4 => self.op_reg_add(reg_x, reg_y),
                0x5 => self.op_reg_sub(reg_x, reg_y),
                0x6 => self.op_shr(reg_x),
                0x7 => self.op_reg_sub_rev(reg_x, reg_y),
                0xe => self.op_shl(reg_x),
                _ => {
                    panic!("Opcode {opcode} is invalid")
                }
            },
            0x9 => {
                assert_eq!(suffix, 0);
                self.op_skip_reg_ne(reg_x, reg_y)
            }
            0xa => self.op_mov_ptr(address),
            0xb => self.op_reg0_jmp(address),
            0xc => self.op_rand(reg_x, constant),
            0xd => self.op_display(reg_x, reg_y, suffix),
            0xf => match constant {
                0x07 => self.op_dump_delay(reg_x),
                0x15 => self.op_set_delay(reg_x),
                0x18 => self.op_set_sound(reg_x),
                0x1e => self.op_ptr_add(reg_x),
                0x29 => self.op_mov_font_addr(reg_x),
                0x33 => self.op_bdc(reg_x),
                0x55 => self.op_reg_dump(reg_x),
                0x65 => self.op_reg_load(reg_x),
                _ => {
                    println!("Opcode {opcode} not implemented yet for F-group, const={constant:x}");
                    self.state = State::Paused;
                }
            },
            _ => {
                println!("Opcode {opcode} not implemented yet ({prefix:x})");
                self.state = State::Paused;
            }
        }
        Ok(())
    }

    fn push(&mut self, value: usize) {
        self.stack[self.sp] = value;
        self.sp += 1;
    }

    fn pop(&mut self) -> usize {
        self.sp -= 1;
        self.stack[self.sp]
    }

    fn op_clear_screen(&mut self) {
        self.video_memory.iter_mut().for_each(|val| *val = 0);
    }

    fn op_return(&mut self) {
        self.pc = self.pop();
    }

    fn op_jmp(&mut self, address: u16) {
        self.pc = address as usize;
    }

    fn op_call(&mut self, address: u16) {
        let ret_address = self.pc;
        self.push(ret_address);
        self.pc = address as usize;
    }

    fn op_skip_eq(&mut self, x: u16, value: u8) {
        if self.reg[x as usize] == value {
            self.pc += 2;
        }
    }

    fn op_skip_ne(&mut self, x: u16, value: u8) {
        if self.reg[x as usize] != value {
            self.pc += 2;
        }
    }

    fn op_skip_reg_eq(&mut self, x: u16, y: u16) {
        if self.reg[x as usize] == self.reg[y as usize] {
            self.pc += 2;
        }
    }

    fn op_mov(&mut self, x: u16, value: u8) {
        self.reg[x as usize] = value;
    }

    fn op_add(&mut self, x: u16, value: u8) {
        let x = x as usize;
        let sum = value as u16 + self.reg[x] as u16;
        self.reg[x] = (sum & 0xff) as u8;
    }

    fn op_reg_mov(&mut self, x: u16, y: u16) {
        self.reg[x as usize] = self.reg[y as usize];
    }

    fn op_or(&mut self, x: u16, y: u16) {
        self.reg[x as usize] |= self.reg[y as usize];
    }

    fn op_and(&mut self, x: u16, y: u16) {
        self.reg[x as usize] &= self.reg[y as usize];
    }

    fn op_xor(&mut self, x: u16, y: u16) {
        self.reg[x as usize] ^= self.reg[y as usize];
    }

    fn op_reg_add(&mut self, x: u16, y: u16) {
        let (x, y) = (x as usize, y as usize);
        let a = self.reg[x] as u16;
        let b = self.reg[y] as u16;
        let sum = a + b;
        self.reg[0xf] = if sum > 0xff { 1 } else { 0 };
        self.reg[x] = (sum & 0xff) as u8;
    }

    fn op_reg_sub(&mut self, x: u16, y: u16) {
        let (x, y) = (x as usize, y as usize);
        let a = self.reg[x] as i16;
        let b = self.reg[y] as i16;
        self.reg[0xf] = if b > a { 1 } else { 0 };
        self.reg[x] = ((a - b) & 0xff) as u8;
    }

    fn op_shr(&mut self, x: u16) {
        let x = x as usize;
        self.reg[0xf] = self.reg[x] & 1;
        self.reg[x] >>= 1;
    }

    fn op_reg_sub_rev(&mut self, x: u16, y: u16) {
        let (x, y) = (x as usize, y as usize);
        let a = self.reg[x] as i16;
        let b = self.reg[y] as i16;
        self.reg[0xf] = if a > b { 1 } else { 0 };
        self.reg[x] = ((b - a) & 0xff) as u8;
    }

    fn op_shl(&mut self, x: u16) {
        let x = x as usize;
        self.reg[0xf] = self.reg[x] >> 7;
        self.reg[x] <<= 1;
    }

    fn op_skip_reg_ne(&mut self, x: u16, y: u16) {
        if self.reg[x as usize] != self.reg[y as usize] {
            self.pc += 2;
        }
    }

    fn op_mov_ptr(&mut self, address: u16) {
        self.reg_ptr = address;
    }

    fn op_reg0_jmp(&mut self, address: u16) {
        self.pc = self.reg[0] as usize + address as usize;
    }

    fn op_rand(&mut self, x: u16, value: u8) {
        let x = x as usize;
        self.reg[x] = value & self.rng.gen::<u8>();
    }

    fn op_display(&mut self, x: u16, y: u16, height: u8) {
        let (x, y, height) = (x as usize, y as usize, height as usize);
        let (row, col) = (self.reg[y] as usize, self.reg[x] as usize);

        let ptr = self.reg_ptr as usize;

        let mut is_flipped = false;
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
                self.video_memory[idx] ^= (val >> (7 - j)) & 1;
                is_flipped |= prev == 1 && self.video_memory[idx] == 0;
            }
        }
        self.reg[0xf] = if is_flipped { 1 } else { 0 };
    }

    fn op_bdc(&mut self, x: u16) {
        let x = x as usize;
        let mut val = self.reg[x];
        let ptr = self.reg_ptr as usize;
        self.memory[ptr + 2] = val % 10;
        val /= 10;
        self.memory[ptr + 1] = val % 10;
        val /= 10;
        self.memory[ptr] = val % 10;
    }

    fn op_reg_dump(&mut self, x: u16) {
        let x = x as usize;
        let ptr = self.reg_ptr as usize;
        for offset in 0..=x {
            self.memory[ptr + offset] = self.reg[offset];
        }
    }

    fn op_reg_load(&mut self, x: u16) {
        let x = x as usize;
        let ptr = self.reg_ptr as usize;
        for offset in 0..=x {
            self.reg[offset] = self.memory[ptr + offset];
        }
    }

    fn op_ptr_add(&mut self, x: u16) {
        let val = self.reg[x as usize];
        self.reg_ptr += val as u16;
    }

    fn op_mov_font_addr(&mut self, x: u16) {
        // ????
        let val = self.reg[x as usize] as u16;
        self.reg_ptr = FONT_START_ADDRESS as u16 + val * 5;
    }

    fn op_set_delay(&mut self, x: u16) {
        self.timer_delay = self.reg[x as usize];
    }

    fn op_set_sound(&mut self, x: u16) {
        self.timer_sound = self.reg[x as usize];
    }

    fn op_dump_delay(&mut self, x: u16) {
        self.reg[x as usize] = self.timer_delay;
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

    pub fn is_delayed(&self) -> bool {
        self.timer_delay > 0
    }
}
