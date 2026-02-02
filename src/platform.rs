use sdl2::{EventPump, event::Event, keyboard::Keycode, render::{Texture, WindowCanvas}};

pub struct Platform {
    pub canvas: WindowCanvas,
    pub event_pump: EventPump,
}

pub fn key_map(key: Keycode) -> Option<usize> {
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
