extern crate sdl2;

use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use std::time::{Duration, Instant};
use std::{env, process};


use std::io::{self, Read, Seek, SeekFrom};

mod chip8;
mod platform;
mod constants;

use chip8::Chip8;
use platform::Platform;
use constants::{VIDEO_WIDTH, VIDEO_HEIGHT, START_ADDRESS};

use crate::chip8::config_chip8_tables;

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

    let texture_creator = platform.canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA8888,
            VIDEO_WIDTH as u32,
            VIDEO_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    let video_pitch = (VIDEO_WIDTH as u32 * 4) as usize;

    let mut last_cycle_time = Instant::now();
    let delay_duration = Duration::from_millis(cycle_delay);

    'gameloop: loop {

        if platform.process_input(&mut chip8.keypad) {
            break 'gameloop;
        }

        let current_time = Instant::now();

        if current_time.duration_since(last_cycle_time) >= delay_duration {
            last_cycle_time = current_time;

            chip8.cycle();

            if chip8.delay_timer > 0 {
                chip8.delay_timer -= 1;
            }
            if chip8.sound_timer > 0 {
                chip8.sound_timer -= 1;
            }

            let buffer_as_u8 = unsafe {
                std::slice::from_raw_parts(
                    chip8.display.as_ptr() as *const u8,
                    chip8.display.len() * 4,
                )
            };

            platform.update(&mut texture, buffer_as_u8, video_pitch)?;
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    Ok(())
}