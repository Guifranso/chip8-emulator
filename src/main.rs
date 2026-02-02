extern crate sdl2;

use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use std::time::{Duration, Instant};
use std::{env, process};

use rand::Rng;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

const START_ADDRESS: u16 = 0x200;
const FONTSET_START_ADDRESS: u16 = 0x50;
const FONTSET_SIZE: u8 = 80;
const VIDEO_WIDTH: u8 = 64;
const VIDEO_HEIGHT: u8 = 32;

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

type OpFunction = fn(&mut Chip8);

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

    //tables
    table: [OpFunction; 16],
    table_0: [OpFunction; 0xE + 1],
    table_8: [OpFunction; 0xE + 1],
    table_e: [OpFunction; 0xE + 1],
    table_f: [OpFunction; 0x65 + 1],
}

fn key_map(key: Keycode) -> Option<usize> {
    match key {
        Keycode::X => Some(0x0),
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::Z => Some(0xA),
        Keycode::C => Some(0xB),
        Keycode::Num4 => Some(0xC),
        Keycode::R => Some(0xD),
        Keycode::F => Some(0xE),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

struct Platform {
    canvas: WindowCanvas,
    event_pump: EventPump,
}

impl Platform {
    pub fn new(title: &str, window_width: u32, window_height: u32) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, window_width, window_height)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .accelerated()
            // .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;

        Ok(Platform { canvas, event_pump })
    }

    pub fn update(
        &mut self,
        texture: &mut Texture,
        buffer: &[u8],
        pitch: usize,
    ) -> Result<(), String> {
        texture
            .update(None, buffer, pitch)
            .map_err(|e| e.to_string())?;

        self.canvas.clear();
        self.canvas.copy(&texture, None, None)?;
        self.canvas.present();

        Ok(())
    }

    pub fn process_input(&mut self, keys: &mut [u8; 16]) -> bool {
        let mut quit = false;

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    quit = true;
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(idx) = key_map(key) {
                        keys[idx] = 1;
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(idx) = key_map(key) {
                        keys[idx] = 0;
                    }
                }
                _ => {}
            }
        }
        quit
    }

}

fn main() -> Result<(), String> {
    unsafe { env::set_var("RUST_BACKTRACE", "1") };

    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <Scale> <Delay> <ROM>", args[0]);
        process::exit(1);
    }

    let video_scale: u32 = args[1].parse().expect("Scale must be a number");
    let cycle_delay: u64 = args[2].parse().expect("Delay must be a number");
    let rom_filename = &args[3];

    let mut platform = Platform::new(
        "CHIP-8 Emulator",
        VIDEO_WIDTH as u32 * video_scale,
        VIDEO_HEIGHT as u32 * video_scale,
    )?;

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

        table: [Chip8::OP_null; 16],
        table_0: [Chip8::OP_null; 0xE + 1],
        table_8: [Chip8::OP_null; 0xE + 1],
        table_e: [Chip8::OP_null; 0xE + 1],
        table_f: [Chip8::OP_null; 0x65 + 1],
    };

    config_chip8_tables(&mut chip8);

    chip8.load_fontset();

    if let Err(e) = chip8.load_rom(rom_filename) {
        eprintln!("Failed to load ROM: {}", e);
        process::exit(1);
    }

    // 3. Preparação do Vídeo (Específico do SDL2 em Rust)
    // Precisamos criar a textura aqui fora do loop por causa do Borrow Checker
    let texture_creator = platform.canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA8888,
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    // Pitch: Em Rust com u32 (RGBA8888), cada pixel tem 4 bytes.
    let video_pitch = (VIDEO_WIDTH as u32 * 4) as usize;

    // 4. Controle de Tempo (std::chrono do C++)
    let mut last_cycle_time = Instant::now();
    let delay_duration = Duration::from_millis(cycle_delay);

    // Loop Principal
    'gameloop: loop {
        // ProcessInput retorna true se for para sair (quit)
        // Passamos o keypad mutável para a plataforma preencher
        if platform.process_input(&mut chip8.keypad) {
            break 'gameloop;
        }

        let current_time = Instant::now();

        // Verifica se passou tempo suficiente (dt > cycleDelay)
        if current_time.duration_since(last_cycle_time) >= delay_duration {
            last_cycle_time = current_time;

            // Roda um ciclo da CPU
            chip8.cycle(); // Ou chip8.tick() / cycle(), dependendo do nome da sua função principal

            // Atualiza os timers (Delay e Sound)
            // No Chip-8 original, os timers rodam a 60Hz independente do clock da CPU.
            // Para simplificar igual ao seu C++, vamos decrementar aqui mesmo:
            if chip8.delay_timer > 0 {
                chip8.delay_timer -= 1;
            }
            if chip8.sound_timer > 0 {
                chip8.sound_timer -= 1;
            }

            // --- Renderização ---
            // O SDL pede um slice de u8 (bytes), mas nosso display é u32.
            // Fazemos uma conversão "unsafe" rápida (igual ao cast de C++) para performance.
            let buffer_as_u8 = unsafe {
                std::slice::from_raw_parts(
                    chip8.display.as_ptr() as *const u8,
                    chip8.display.len() * 4, // * 4 porque u32 são 4 bytes
                )
            };

            platform.update(&mut texture, buffer_as_u8, video_pitch)?;
        }

        // Uma pequena pausa para não fritar a CPU do computador (opcional, mas recomendado)
        std::thread::sleep(Duration::from_micros(100));
    }

    Ok(())
}

fn config_chip8_tables(chip8: &mut Chip8) {
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
    fn load_rom(&mut self, file_path: &str) -> io::Result<()> {
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

    fn load_fontset(&mut self) {
        for i in 0..FONTSET_SIZE as usize {
            self.memory[FONTSET_START_ADDRESS as usize + i] = fontset[i];
        }
    }

    fn cycle(&mut self) {
        self.opcode = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[(self.pc + 1) as usize]) as u16;

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
    fn OP_00E0(&mut self) {
        //self.display = [0; 64 * 32];
        self.display.fill(0);
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

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    //SNE(skip if not equal) Vx, byte
    fn OP_4xkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] != byte {
            self.pc += 2;
        }
    }

    //SE Vx, Vy
    fn OP_5xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] == self.registers[vy as usize] {
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

        self.registers[vx as usize] = self.registers[vx as usize].wrapping_add(byte);
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

        if self.registers[vx as usize] >= self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        //In the background, apply the operation sum & 0xFF
        self.registers[vx as usize] =  self.registers[vx as usize].wrapping_sub(self.registers[vy as usize]);
    }

    //SHR Vx
    fn OP_8xy6(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[0xF] = self.registers[vx as usize] & 0x1;

        self.registers[vx as usize] >>= 1;
    }

    //SUBN Vx, Vy
    fn OP_8xy7(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] >= self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] = self.registers[vy as usize].wrapping_sub(self.registers[vx as usize]);
    }

    //SHL Vx {, Vy}
    fn OP_8xyE(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[0xF] = self.registers[vx as usize] & 0x80 >> 7;

        self.registers[vx as usize] <<= 1;
    }

    //SNE Vx, Vy
    fn OP_9xy0(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    //LD I, addr
    // I = nnn
    fn OP_Annn(&mut self) {
        let address: u32 = (self.opcode & 0x0FFF) as u32;

        self.index = address;
    }

    //JP V0, addr
    fn OP_Bnnn(&mut self) {
        let address: u32 = (self.opcode & 0x0FFF) as u32;

        self.pc = self.registers[0] as u16 + address as u16;
    }

    //RND Vx, byte
    fn OP_Cxkk(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        let mut rng = rand::rng();
        self.registers[vx as usize] = rng.random::<u8>() & byte;
    }

    //DRW Vx, Vy, nibble
    //
    fn OP_Dxyn(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let vy: u8 = ((self.opcode & 0x00F0) >> 4) as u8;
        let height: u8 = (self.opcode & 0x000F) as u8;

        let x_pos: usize = (self.registers[vx as usize] % VIDEO_WIDTH) as usize;
        let y_pos: usize = (self.registers[vy as usize] % VIDEO_HEIGHT) as usize;

        self.registers[0xF] = 0;

        for row in 0..height as usize {

            if y_pos + row > VIDEO_HEIGHT as usize {
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
    fn OP_Ex9E(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] != 0 {
            self.pc += 2;
        }
    }

    //SKNP Vx
    fn OP_ExA1(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] == 0 {
            self.pc += 2;
        }
    }

    //LD Vx, DT
    fn OP_Fx07(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.registers[vx as usize] = self.delay_timer;
    }

    //LD Vx, K
    fn OP_Fx0A(&mut self) {
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
    fn OP_Fx15(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.delay_timer = self.registers[vx as usize];
    }

    //LD ST, Vx
    fn OP_Fx18(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.sound_timer = self.registers[vx as usize];
    }

    //ADD I, Vx
    fn OP_Fx1E(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        self.index += self.registers[vx as usize] as u32;
    }

    //LD F, Vx
    fn OP_Fx29(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let digit = self.registers[vx as usize];

        self.index = FONTSET_START_ADDRESS as u32 + (5 * digit as u32);
    }

    //LD B, Vx
    fn OP_Fx33(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
        let mut value = self.registers[vx as usize];

        self.memory[(self.index + 2) as usize] = value % 10;
        value /= 10;

        self.memory[(self.index + 1) as usize] = value % 10;
        value /= 10;

        self.memory[self.index as usize] = value % 10;
    }

    //LD [I], Vx
    fn OP_Fx55(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        for i in 0..=vx as usize {
            self.memory[self.index as usize + i] = self.registers[i];
        }
    }

    //LD Vx, [I]
    fn OP_Fx65(&mut self) {
        let vx: u8 = ((self.opcode & 0x0F00) >> 8) as u8;

        for i in 0..=vx as usize {
            self.registers[i] = self.memory[self.index as usize + i];
        }
    }

    fn OP_null(&mut self) {}
}

//Table functions
impl Chip8 {
    fn table_0_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_0[index];
        op_function(self);
    }

    fn table_8_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_8[index];
        op_function(self);
    }

    fn table_e_fn(&mut self) {
        let index = (self.opcode & 0x000F) as usize;
        let op_function = self.table_e[index];
        op_function(self);
    }

    fn table_f_fn(&mut self) {
        let index = (self.opcode & 0x00FF) as usize;
        let op_function = self.table_f[index];
        op_function(self);
    }
}
