use rand::Rng;
use rand::prelude::*;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

const START_ADDRESS: u16 = 0x200;
const FONTSET_START_ADDRESS: u16 = 0x50;
const FONTSET_SIZE: u8 = 80;

const fontset: [u8; FONTSET_SIZE as usize] = [
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

fn main() {
    let mut chip8 = Chip8 {
        registers: [0; 16],
        memory: [0; 4096],
        index: 0,
        pc: START_ADDRESS,
        stack: [0; 16],
        sp: 0,
        delay_timer: 0,
        sound_timer: 0,
        keypad: [0; 16],
        display: [0; 64 * 32],
        opcode: 0,
    };

    let _ = chip8.load_rom("./text.txt");

    chip8.load_fontset();

    chip8.print_memory();

    let mut rng = rand::rng();

    let mut random_byte = rng.random::<u8>();
}

#[derive(Debug)]
struct Chip8 {
    registers: [u8; 16],
    memory: [u8; 4096],
    index: u32,
    pc: u16,
    stack: [u16; 16],
    sp: u16,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [u8; 16],
    display: [u32; 64 * 32],
    opcode: u16,
}

impl Chip8 {
    fn load_rom(&mut self, file_path: &str) -> io::Result<()> {
        // Open the file and go to the last position, to get the file size
        // and then goes back to the first position.
        let mut file = File::open(file_path)?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        //save the binary value in the buffer
        let mut buffer: Vec<u8> = vec![0; size as usize];
        file.read_exact(&mut buffer)?;

        println!("File size: {}", size);
        println!("Buffer value: \n {:?}", &buffer);

        //copy the buffer value to the memory
        for (i, byte) in buffer.iter().enumerate() {
            self.memory[START_ADDRESS as usize + i] = *byte;
        }

        Ok(())
    }

    fn load_fontset(&mut self) {
        for i in 0..FONTSET_SIZE as usize {
            self.memory[FONTSET_START_ADDRESS as usize + i] = fontset[i];
        }
    }

    fn print_memory(&mut self) {
        println!("Memory value: \n {:?}", self.memory);
    }
}

//Instructions
impl Chip8 {
    //CLS
    fn OP_00E0(&mut self) {
        //definir todos os valores da area de display para zero
        //memset(video, 0, sizeof(video));
    }

    //RET
    fn OP_00EE(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    //JP addr
    fn OP_1nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF as u16;

        self.pc = address;
    }

    //CALL addr
    fn OP_2nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF as u16;

        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;

        self.pc = address;
    }

    //SE(skip if equal) Vx, byte
    fn OP_3xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if (self.registers[vx as usize] == byte) {
            self.pc += 2;
        }
    }

    //SNE(skip if not equal) Vx, byte
    fn OP_4xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if (self.registers[vx as usize] != byte) {
            self.pc += 2;
        }
    }

    //SE Vx, Vy
    fn OP_5xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if (self.registers[vx as usize] == self.registers[vy as usize]) {
            self.pc += 2;
        }
    }

    //LD Vx, byte
    fn OP_6xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = byte;
    }

    //ADD Vx, byte
    fn OP_7xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] += byte;
    }

    //LD Vx, Vy
    fn OP_8xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    //OR Vx, Vy
    fn OP_8xy1(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    //AND Vx, Vy
    fn OP_8xy2(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    //XOR Vx, Vy
    fn OP_8xy3(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    //ADD Vx, Vy
    fn OP_8xy4(&mut self) {
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
    fn OP_8xy5(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        let sum: u16 = self.registers[vx as usize] as u16 - self.registers[vy as usize] as u16;

        if self.registers[vx as usize] > self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        //In the background, apply the operation sum & 0xFF
        self.registers[vx as usize] -= self.registers[vy as usize];
    }
}
