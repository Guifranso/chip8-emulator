use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use rand::Rng;

use crate::constants::{FONTSET, FONTSET_SIZE, FONTSET_START_ADDRESS, START_ADDRESS, VIDEO_HEIGHT, VIDEO_WIDTH};

#[derive(Debug)]
pub struct Chip8 {
    pub registers: [u8; 16],
    pub memory: [u8; 4096],
    pub index: u32,
    pub pc: u16,
    pub stack: [u16; 16],
    pub sp: u16,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub keypad: [u8; 16],
    pub display: [u32; 64 * 32],
    pub opcode: u16,

    //tables
    pub table: [OpFunction; 16],
    pub table_0: [OpFunction; 0xE + 1],
    pub table_8: [OpFunction; 0xE + 1],
    pub table_e: [OpFunction; 0xE + 1],
    pub table_f: [OpFunction; 0x65 + 1],
}

pub type OpFunction = fn(&mut Chip8);

pub fn config_chip8_tables(chip8: &mut Chip8) {
    chip8.table[0x0] = Chip8::table_0_fn;
    chip8.table[0x1] = Chip8::OP_1nnn;
    chip8.table[0x2] = Chip8::OP_2nnn;
    chip8.table[0x3] = Chip8::OP_3xkk;
    chip8.table[0x4] = Chip8::OP_4xkk;
    chip8.table[0x5] = Chip8::OP_5xy0;
    chip8.table[0x6] = Chip8::OP_6xkk;
    chip8.table[0x7] = Chip8::OP_7xkk;
    chip8.table[0x8] = Chip8::table_8_fn;
    chip8.table[0x9] = Chip8::OP_9xy0;
    chip8.table[0xA] = Chip8::OP_Annn;
    chip8.table[0xB] = Chip8::OP_Bnnn;
    chip8.table[0xC] = Chip8::OP_Cxkk;
    chip8.table[0xD] = Chip8::OP_Dxyn;
    chip8.table[0xE] = Chip8::table_e_fn;
    chip8.table[0xF] = Chip8::table_f_fn;

    for i in 0..=0xE {
        chip8.table_0[i] = Chip8::OP_null;
        chip8.table_8[i] = Chip8::OP_null;
        chip8.table_e[i] = Chip8::OP_null;
    }

    chip8.table_0[0x0] = Chip8::OP_00E0;
    chip8.table_0[0xE] = Chip8::OP_00EE;

    chip8.table_8[0x0] = Chip8::OP_8xy0;
    chip8.table_8[0x1] = Chip8::OP_8xy1;
    chip8.table_8[0x2] = Chip8::OP_8xy2;
    chip8.table_8[0x3] = Chip8::OP_8xy3;
    chip8.table_8[0x4] = Chip8::OP_8xy4;
    chip8.table_8[0x5] = Chip8::OP_8xy5;
    chip8.table_8[0x6] = Chip8::OP_8xy6;
    chip8.table_8[0x7] = Chip8::OP_8xy7;
    chip8.table_8[0xE] = Chip8::OP_8xyE;

    chip8.table_e[0x1] = Chip8::OP_ExA1;
    chip8.table_e[0xE] = Chip8::OP_Ex9E;

    for i in 0..=0x65 {
        chip8.table_f[i as usize] = Chip8::OP_null;
    }

    chip8.table_f[0x07] = Chip8::OP_Fx07;
    chip8.table_f[0x0A] = Chip8::OP_Fx0A;
    chip8.table_f[0x15] = Chip8::OP_Fx15;
    chip8.table_f[0x18] = Chip8::OP_Fx18;
    chip8.table_f[0x1E] = Chip8::OP_Fx1E;
    chip8.table_f[0x29] = Chip8::OP_Fx29;
    chip8.table_f[0x33] = Chip8::OP_Fx33;
    chip8.table_f[0x55] = Chip8::OP_Fx55;
    chip8.table_f[0x65] = Chip8::OP_Fx65;
}

//Main functions
impl Chip8 {
    pub fn load_rom(&mut self, file_path: &str) -> io::Result<()> {
        // Open the file and go to the last position, to get the file size
        // and then goes back to the first position.
        let mut file = File::open(file_path)?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        //Save the binary value in the buffer
        let mut buffer: Vec<u8> = vec![0; size as usize];
        file.read_exact(&mut buffer)?;

        //Copy the buffer value to the memory
        for (i, byte) in buffer.iter().enumerate() {
            self.memory[START_ADDRESS as usize + i] = *byte;
        }

        Ok(())
    }

    pub fn load_fontset(&mut self) {
        for i in 0..FONTSET_SIZE as usize {
            self.memory[FONTSET_START_ADDRESS as usize + i] = FONTSET[i];
        }
    }

    pub fn cycle(&mut self) {
        self.opcode = (self.memory[self.pc as usize] as u16) << 8
            | (self.memory[(self.pc + 1) as usize]) as u16;

        self.pc += 2;

        self.table[((self.opcode & 0xF000) >> 12) as usize](self);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}

//Instructions
impl Chip8 {
    //CLS
    pub fn OP_00E0(&mut self) {
        //self.display = [0; 64 * 32];
        self.display.fill(0);
    }

    //RET
    pub fn OP_00EE(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    //JP addr
    pub fn OP_1nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF as u16;

        self.pc = address;
    }

    //CALL addr
    pub fn OP_2nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF as u16;

        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;

        self.pc = address;
    }

    //SE(skip if equal) Vx, byte
    pub fn OP_3xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    //SNE(skip if not equal) Vx, byte
    pub fn OP_4xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] != byte {
            self.pc += 2;
        }
    }

    //SE Vx, Vy
    pub fn OP_5xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] == self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    //LD Vx, byte
    pub fn OP_6xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = byte;
    }

    //ADD Vx, byte
    pub fn OP_7xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = self.registers[vx as usize].wrapping_add(byte);
    }

    //LD Vx, Vy
    pub fn OP_8xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    //OR Vx, Vy
    pub fn OP_8xy1(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    //AND Vx, Vy
    pub fn OP_8xy2(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    //XOR Vx, Vy
    pub fn OP_8xy3(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    //ADD Vx, Vy
    pub fn OP_8xy4(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        let sum: u16 = self.registers[vx as usize] as u16 + self.registers[vy as usize] as u16;

        if sum > 255 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        //In the background, apply the operation sum & 0xFF
        self.registers[vx as usize] = sum as u8;
    }

    //SUB Vx, Vy
    pub fn OP_8xy5(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] >= self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        //In the background, apply the operation sum & 0xFF
        self.registers[vx as usize] =
            self.registers[vx as usize].wrapping_sub(self.registers[vy as usize]);
    }

    //SHR Vx
    pub fn OP_8xy6(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[0xF] = self.registers[vx as usize] & 0x1;

        self.registers[vx as usize] >>= 1;
    }

    //SUBN Vx, Vy
    pub fn OP_8xy7(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] >= self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] =
            self.registers[vy as usize].wrapping_sub(self.registers[vx as usize]);
    }

    //SHL Vx {, Vy}
    pub fn OP_8xyE(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[0xF] = self.registers[vx as usize] & 0x80 >> 7;

        self.registers[vx as usize] <<= 1;
    }

    //SNE Vx, Vy
    pub fn OP_9xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    //LD I, addr
    // I = nnn
    pub fn OP_Annn(&mut self) {
        let address: u32 = (self.opcode & 0x0FFF) as u32;

        self.index = address;
    }

    //JP V0, addr
    pub fn OP_Bnnn(&mut self) {
        let address: u32 = (self.opcode & 0x0FFF) as u32;

        self.pc = self.registers[0] as u16 + address as u16;
    }

    //RND Vx, byte
    pub fn OP_Cxkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        let mut rng = rand::rng();
        self.registers[vx as usize] = rng.random::<u8>() & byte;
    }

    //DRW Vx, Vy, nibble
    //
    pub fn OP_Dxyn(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;
        let height: u8 = (self.opcode & 0x000F) as u8;

        let x_pos: usize = (self.registers[vx as usize] % VIDEO_WIDTH) as usize;
        let y_pos: usize = (self.registers[vy as usize] % VIDEO_HEIGHT) as usize;

        self.registers[0xF] = 0;

        for row in 0..height as usize {
            if y_pos + row >= VIDEO_HEIGHT as usize {
                break;
            }

            let sprite_byte = self.memory[self.index as usize + row];

            for column in 0..8 as usize {
                let sprite_pixel = sprite_byte & (0x80 >> column);

                if sprite_pixel != 0 {
                    let screen_pixel =
                        &mut self.display[(y_pos + row) * VIDEO_WIDTH as usize + (x_pos + column)];

                    if *screen_pixel == 0xFFFFFFFF {
                        self.registers[0xF] = 1;
                    }

                    *screen_pixel ^= 0xFFFFFFFF;
                }
            }
        }
    }

    //SKP Vx
    pub fn OP_Ex9E(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] != 0 {
            self.pc += 2;
        }
    }

    //SKNP Vx
    pub fn OP_ExA1(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] == 0 {
            self.pc += 2;
        }
    }

    //LD Vx, DT
    pub fn OP_Fx07(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[vx as usize] = self.delay_timer;
    }

    //LD Vx, K
    pub fn OP_Fx0A(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        if self.keypad[0] != 0 {
            self.registers[vx as usize] = 0;
        } else if self.keypad[1] != 0 {
            self.registers[vx as usize] = 1;
        } else if self.keypad[2] != 0 {
            self.registers[vx as usize] = 2;
        } else if self.keypad[3] != 0 {
            self.registers[vx as usize] = 3;
        } else if self.keypad[4] != 0 {
            self.registers[vx as usize] = 4;
        } else if self.keypad[5] != 0 {
            self.registers[vx as usize] = 5;
        } else if self.keypad[6] != 0 {
            self.registers[vx as usize] = 6;
        } else if self.keypad[7] != 0 {
            self.registers[vx as usize] = 7;
        } else if self.keypad[8] != 0 {
            self.registers[vx as usize] = 8;
        } else if self.keypad[9] != 0 {
            self.registers[vx as usize] = 9;
        } else if self.keypad[10] != 0 {
            self.registers[vx as usize] = 10;
        } else if self.keypad[11] != 0 {
            self.registers[vx as usize] = 11;
        } else if self.keypad[12] != 0 {
            self.registers[vx as usize] = 12;
        } else if self.keypad[13] != 0 {
            self.registers[vx as usize] = 13;
        } else if self.keypad[14] != 0 {
            self.registers[vx as usize] = 14;
        } else if self.keypad[15] != 0 {
            self.registers[vx as usize] = 15;
        } else {
            self.pc -= 2;
        }
    }

    //LD DT, Vx
    pub fn OP_Fx15(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.delay_timer = self.registers[vx as usize];
    }

    //LD ST, Vx
    pub fn OP_Fx18(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.sound_timer = self.registers[vx as usize];
    }

    //ADD I, Vx
    pub fn OP_Fx1E(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.index += self.registers[vx as usize] as u32;
    }

    //LD F, Vx
    pub fn OP_Fx29(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let digit = self.registers[vx as usize];

        self.index = FONTSET_START_ADDRESS as u32 + (5 * digit as u32);
    }

    //LD B, Vx
    pub fn OP_Fx33(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let mut value = self.registers[vx as usize];

        self.memory[(self.index + 2) as usize] = value % 10;
        value /= 10;

        self.memory[(self.index + 1) as usize] = value % 10;
        value /= 10;

        self.memory[self.index as usize] = value % 10;
    }

    //LD [I], Vx
    pub fn OP_Fx55(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        for i in 0..=vx as usize {
            self.memory[self.index as usize + i] = self.registers[i];
        }
    }

    //LD Vx, [I]
    pub fn OP_Fx65(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        for i in 0..=vx as usize {
            self.registers[i] = self.memory[self.index as usize + i];
        }
    }

    pub fn OP_null(&mut self) {}
}

//Table functions
impl Chip8 {
    pub fn table_0_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_0[index];
        op_function(self);
    }

    pub fn table_8_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_8[index];
        op_function(self);
    }

    pub fn table_e_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_e[index];
        op_function(self);
    }

    pub fn table_f_fn(&mut self) {
        let index = (self.opcode & 0x00FF) as usize;
        let op_function = self.table_f[index];
        op_function(self);
    }
}
