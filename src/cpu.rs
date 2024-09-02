//! Huge thanks to:
//! - http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
//! - https://tobiasvl.github.io/blog/write-a-chip-8-emulator
//! - https://www.freecodecamp.org/news/creating-your-very-own-chip-8-emulator
//!
//! TODO:
//! - handle program end

use crate::font::{CHIP_FONT, CHIP_FONT_LEN};

/// CHIP-8 display width
pub const DISPLAY_WIDTH: u32 = 64;
/// CHIP-8 display height
pub const DISPLAY_HEIGHT: u32 = 32;
/// Length of the display 1D array
pub const DISPLAY_DATA_LEN: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize;
/// Max memory size
const MEMORY_CAPACITY: usize = 4096;
/// Max stack size
const STACK_CAPACITY: usize = 16;

/// Starting address of the program in the memory
const START_PC: u16 = 0x200;

/// CHIP-8 cpu
pub struct Cpu {
    /// Whether the game is loaded
    ready: bool,

    /// V*x* registers - where *x* is a hex digit from `0x0` through `0xF`
    v: [u8; 16],
    /// I register - points to an address/index in the memory
    i: u16,
    /// Program counter. Represents a currently executing address
    pc: u16,
    /// Stack pointer. Points to the topmost (last) level of the stack
    sp: u8,

    /// DT register - delay timer
    dt: u8,
    /// ST register - sound timer
    pub st: u8,

    /// List of adresses to which the interpreter should return after finishing with a subroutine
    stack: [u16; STACK_CAPACITY],
    memory: [u8; MEMORY_CAPACITY],
    /// 1D array of each pixel state (on/off)
    pub display: [bool; DISPLAY_DATA_LEN],

    /// Increases by 1 on every step. Mostly used for random number generation
    tick: u16,
    /// Whether to increase the program counter by 2 or not
    jump_next: bool,
    pub display_changed: bool,

    /// Whether is waiting for a button press for Vx
    waiting_button_for: Option<u8>,
    /// Represents the pressed state of all 16 buttons
    buttons: [bool; 16],
}
impl Cpu {
    /// Load a game from binary
    pub fn load(&mut self, bytes: &[u8]) {
        let start = START_PC as usize;

        // Store the game into the memory from 0x200 to 0x200 + game_length
        self.memory[start..start + bytes.len()].copy_from_slice(bytes);

        self.ready = true;
    }
    /// Reset everything
    pub fn unload(&mut self) {
        *self = Self::default();
        self.ready = false;
    }
    /// Reset CPU state, but leave memory untouched
    pub fn restart(&mut self) {
        *self = Self {
            memory: self.memory,
            ..Default::default()
        }
    }

    /// Returns whether the cpu updated or not
    pub fn step(&mut self) {
        // Step only if it is not waiting for a button press
        if self.waiting_button_for.is_some() {
            return;
        }
        let pc = self.pc as usize;

        self.tick = self.tick.wrapping_add(1);

        // We need to shift the first byte by 8 bits left so we can "concatenate"
        // it with the second byte
        //
        // For example:
        // 0xAB << 8 -> 0xAB00
        // 0xAB00 | 0x12 -> 0xAB12
        let ins = ((self.memory[pc] as u16) << 8) | self.memory[pc + 1] as u16;

        self.execute(ins);

        if self.jump_next {
            // Increase by 2 because each instruction consists of 2 bytes
            self.pc += 2;
        }
        self.jump_next = true;
    }
    pub fn step_timers(&mut self) {
        // Decrement times
        self.dt = self.dt.saturating_sub(1);
        self.st = self.st.saturating_sub(1);
    }
    pub fn button_pressed(&mut self, btn: u8) {
        self.buttons[btn as usize] = true;

        // While waiting for a keypress, a key was pressed
        if let Some(wait_for) = self.waiting_button_for {
            self.set(wait_for, btn);
            self.waiting_button_for = None;
        }
    }
    pub fn button_released(&mut self, btn: u8) {
        self.buttons[btn as usize] = false;
    }

    fn is_btn_pressed(&mut self, btn: u8) -> bool {
        self.buttons[btn as usize]
    }

    /// Execute an instruction
    fn execute(&mut self, ins: u16) {
        let a = (ins & 0xF000) >> 12;
        let b = (ins & 0x0F00) >> 8;
        let c = (ins & 0x00F0) >> 4;
        let d = ins & 0x000F;

        // NNN
        let addr = ins & 0x0FFF;
        // KK
        let byte: u8 = (ins & 0x00FF) as u8;
        // N
        let nibble: u8 = (ins & 0x000F) as u8;
        // Vx register
        let x = b as u8;
        // Vy register
        let y = c as u8;

        match (a, b, c, d) {
            // Clear the display
            (0, 0, 0xE, 0) => self.clear(),
            // Draw a N-byte sprite at Vx and Vy
            (0xD, _, _, _) => self.draw(x, y, nibble),

            // Jump to NNN
            (0x1, _, _, _) => self.jump(addr),
            // Jump to NNN + V0
            (0xB, _, _, _) => self.jump(addr + self.get(0) as u16),
            // Jump to a subroutine
            (0x2, _, _, _) => self.call(addr),
            // Return from a subroutine
            (0, 0, 0xE, 0xE) => self.ret(),

            // Skip if Vx == KK
            (0x3, _, _, _) => self.skip_vx_eq_byte(x, byte),
            // Skip if Vx != KK
            (0x4, _, _, _) => self.skip_vx_neq_byte(x, byte),
            // Skip if Vx == Vy
            (0x5, _, _, _) => self.skip_vx_eq_vy(x, y),
            // Skip if Vx != Vy
            (0x9, _, _, 0) => self.skip_vx_neq_vy(x, y),

            // Vx = KK
            (0x6, _, _, _) => { self.set(x, byte); },
            // Vx += KK
            (0x7, _, _, _) => self.add_vx_byte(x, byte),
            // Vx = Vy
            (0x8, _, _, 0) => self.set_vx_vy(x, y),
            // Vx += Vy
            (0x8, _, _, 4) => self.add_vx_vy(x, y),
            // Vx = Vx - Vy
            (0x8, _, _, 5) => self.sub_vx_vy(x, y),
            // Vx = Vy - Vx
            (0x8, _, _, 7) => self.sub_vy_vx(y, x),
            // Vx = Vx | Vy
            (0x8, _, _, 1) => self.or(x, y),
            // Vx = Vx & Vy
            (0x8, _, _, 2) => self.and(x, y),
            // Vx = Vx ^ Vy
            (0x8, _, _, 3) => self.xor(x, y),
            // Vx = Vx >> 1
            (0x8, _, _, 6) => self.shift_right(x),
            // Vx = Vx << 1
            (0x8, _, _, 0xE) => self.shift_left(x),
            // Vx = random_number & KK
            (0xC, _, _, _) => self.rand(x, byte),

            // Vx = DT
            (0xF, _, 0, 0x7) => { self.set(x, self.dt); },
            // DT = Vx
            (0xF, _, 0x1, 0x5) => self.dt = self.get(x),
            // ST = Vx
            (0xF, _, 0x1, 0x8) => self.st = self.get(x),

            // I = NNN
            (0xA, _, _, _) => self.i = addr,
            // I += Vx
            (0xF, _, 0x1, 0xE) => self.add_i_vx(x),
            // I = Vx * 5
            (0xF, _, 0x2, 0x9) => self.set_i_sprite(x),

            // Skip if Vx is pressed
            (0xE, _, 0x9, 0xE) => self.skip_pressed(x),
            // Skip if Vx is NOT pressed
            (0xE, _, 0xA, 0x1) => self.skip_not_pressed(x),
            // Wait for key press and store it in Vx
            (0xF, _, 0, 0xA) => self.wait_for_keypress(x),

            // Store BCD of Vx
            (0xF, _, 0x3, 0x3) => self.store_bcd(x),
            // Store V0 through Vx to memory starting from I
            (0xF, _, 0x5, 0x5) => self.store_through(x),
            // Read to V0 through Vx from memory starting from I
            (0xF, _, 0x6, 0x5) => self.read_through(x),

            (0, _, _, _) => (/* ignore "jump to sys addr" */),
            _ => (/* ignore unknown instructions for now */)
        }
    }

    /// Get register Vx
    pub fn get(&self, x: u8) -> u8 {
        self.v[x as usize]
    }
    /// Set register Vx
    pub fn set(&mut self, x: u8, value: u8) -> u8 {
        self.v[x as usize] = value;
        value
    }

    // Instructions
    fn clear(&mut self) {
        self.display.fill(false);
        self.display_changed = true;
    }
    fn draw(&mut self, x: u8, y: u8, n: u8) {
        let vx = self.get(x) as usize;
        let vy = self.get(y) as usize;
        let sw = DISPLAY_WIDTH as usize;
        let sh = DISPLAY_HEIGHT as usize;

        let mut overlaps = false;

        for row in 0..n as usize {
            let mut sprite = self.memory[self.i as usize + row];

            for col in 0..8 {
                // Check if a pixel exists in the sprite or not
                if sprite & 0x80 != 0 {
                    let x = (vx + col) % sw;
                    let y = (vy + row) % sh;
                    let idx = y * sw + x;

                    // Flip a pixel on the display
                    if self.display[idx] {
                        self.display[idx] = false;
                        overlaps = true;
                    } else {
                        self.display[idx] = true;
                    }
                }

                // Shift the sprite's pixels left so we can get next pixel in the sprite
                sprite <<= 1;
            }
        }

        self.set(0xF, u8::from(overlaps));
        self.display_changed = true;
    }

    fn jump(&mut self, addr: u16) {
        self.pc = addr;
        self.jump_next = false;
    }
    fn call(&mut self, addr: u16) {
        // Store current program counter into the stack and increase stack pointer
        self.stack[self.sp as usize] = self.pc;
        self.sp = (self.sp + 1).min(STACK_CAPACITY as u8 - 1);

        // Set program counter to subroutine addr
        self.jump(addr)
    }
    fn ret(&mut self) {
        self.sp = self.sp.saturating_sub(1);
        self.pc = self.stack[self.sp as usize];
    }

    fn skip_vx_eq_byte(&mut self, x: u8, byte: u8) {
        if self.get(x) == byte {
            self.pc += 2;
        }
    }
    fn skip_vx_neq_byte(&mut self, x: u8, byte: u8) {
        if self.get(x) != byte {
            self.pc += 2;
        }
    }
    fn skip_vx_eq_vy(&mut self, x: u8, y: u8) {
        if self.get(x) == self.get(y) {
            self.pc += 2;
        }
    }
    fn skip_vx_neq_vy(&mut self, x: u8, y: u8) {
        if self.get(x) != self.get(y) {
            self.pc += 2;
        }
    }

    fn add_vx_byte(&mut self, x: u8, byte: u8) {
        self.set(x, self.get(x).wrapping_add(byte));
    }
    fn set_vx_vy(&mut self, x: u8, y: u8) {
        self.set(x, self.get(y));
    }
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let (val, overflow) = self.get(x).overflowing_add(self.get(y));

        self.set(x, val);
        self.set(0xF, u8::from(overflow));
    }
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let (value, underflow) = self.get(x).overflowing_sub(self.get(y));

        self.set(x, value);
        self.set(0xF, u8::from(!underflow));
    }
    fn sub_vy_vx(&mut self, y: u8, x: u8) {
        let (value, underflow) = self.get(y).overflowing_sub(self.get(x));

        self.set(x, value);
        self.set(0xF, u8::from(!underflow));
    }
    fn or(&mut self, x: u8, y: u8) {
        self.set(x, self.get(x) | self.get(y));
    }
    fn and(&mut self, x: u8, y: u8) {
        self.set(x, self.get(x) & self.get(y));
    }
    fn xor(&mut self, x: u8, y: u8) {
        self.set(x, self.get(x) ^ self.get(y));
    }
    fn shift_right(&mut self, x: u8) {
        self.set(0xF, self.get(x) & 0x1);
        self.v[x as usize] >>= 1;
    }
    fn shift_left(&mut self, x: u8) {
        self.set(0xF, self.get(x) & 0x80);
        self.v[x as usize] <<= 1;
    }
    fn rand(&mut self, x: u8, byte: u8) {
        // "Generates" pseudo random number 0..=255 based on tick
        // Jump a bunch of random math and we have a random number!
        let num = (((self.tick as f32 * 99999.0).sin() + 1.0) / 2.0 * 255.0) as u8;
        self.set(x, num & byte);
    }

    fn add_i_vx(&mut self, x: u8) {
        self.i = self.i.wrapping_add(self.get(x) as u16);
    }
    fn set_i_sprite(&mut self, x: u8) {
        self.i = self.get(x) as u16 * 5;
    }

    fn skip_pressed(&mut self, x: u8) {
        let btn = self.get(x);
        if self.is_btn_pressed(btn) {
            self.pc += 2;
        }
    }
    fn skip_not_pressed(&mut self, x: u8) {
        let btn = self.get(x);
        if !self.is_btn_pressed(btn) {
            self.pc += 2;
        }
    }
    fn wait_for_keypress(&mut self, x: u8) {
        self.waiting_button_for = Some(x);
    }

    /// Stores hundreds of Vx in I, tens of Vx in I+1 and ones of Vx in I+2
    /// For example:
    /// Vx = 230
    /// I = 2
    /// I+1 = 3
    /// I+2 = 0
    fn store_bcd(&mut self, x: u8) {
        let vx = self.get(x);
        let i = self.i as usize;

        self.memory[i] = vx / 100; // Hundreds
        self.memory[i + 1] = (vx % 100) / 10; // Tens
        self.memory[i + 2] = vx % 10; // Ones
    }
    /// Store registers V0 through Vx in memory starting from I
    fn store_through(&mut self, x: u8) {
        for xx in 0..=x {
            self.memory[self.i as usize + xx as usize] = self.get(xx);
        }
    }
    /// Read in registers V0 through Vx from memory starting from I
    fn read_through(&mut self, x: u8) {
        for xx in 0..=x {
            self.set(xx, self.memory[self.i as usize + xx as usize]);
        }
    }
}
impl Default for Cpu {
    fn default() -> Self {
        let mut memory = [0u8; MEMORY_CAPACITY];

        // Store the font into the memory from 0x0 to font_length
        memory[..CHIP_FONT_LEN].copy_from_slice(&CHIP_FONT);

        Self {
            ready: false,

            v: [0; 16],
            i: 0,
            pc: START_PC,
            sp: 0,

            dt: 0,
            st: 0,

            stack: [0; STACK_CAPACITY],
            memory,
            display: [false; DISPLAY_DATA_LEN],

            tick: 0,
            jump_next: true,
            display_changed: false,

            buttons: [false; 16],
            waiting_button_for: None,
        }
    }
}
