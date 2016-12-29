#![feature(proc_macro)]

extern crate timer;
extern crate chrono;

#[macro_use]
extern crate log;
extern crate log4rs;
extern crate log_panics;

#[macro_use]
extern crate serde_derive;

extern crate rand;
extern crate serde;
extern crate serde_yaml;

extern crate ansi_term;

use std::default::Default;

use std::fs::File;
use std::io::Read;
use std::env;
use std::fmt;
use std::collections::HashMap;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;

extern crate sdl2;
use sdl2::render::Renderer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

const ON_COLOR: Color = Color::RGB(255, 0, 0);
const OFF_COLOR: Color = Color::RGB(0, 0, 0);

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 320;

const X_SCALE: u32 = WINDOW_WIDTH / 64;
const Y_SCALE: u32 = WINDOW_HEIGHT / 32;

const DEFAULT_CYCLES_PER_SECOND: u32 = 1000;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = match self.phase {
                0.0...0.5 => self.volume,
                _ => -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}


#[derive(Default, Serialize, Deserialize)]
struct CPU {
    v: [u8; 16],
    i: u16,
    dt: u8,
    st: u8,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "v : ").unwrap();
        for v in &self.v {
            write!(f, "{:x}, ", v).unwrap();
        }
        write!(f, "\n").unwrap();

        write!(f, "i : {:x}\n", self.i).unwrap();
        write!(f, "dt: {:x}\n", self.dt).unwrap();
        write!(f, "st: {:x}\n", self.st).unwrap();
        write!(f, "pc: {:x}\n", self.pc).unwrap();
        write!(f, "sp: {:x}\n", self.sp).unwrap();

        write!(f, "sk: ").unwrap();
        for s in &self.stack {
            write!(f, "{:x}, ", s).unwrap();
        }
        write!(f, "")
    }
}

struct Computer {
    ram: [u8; 4096],
    cpu: CPU,
    keyboard: [bool; 16]
}

impl Default for Computer {
     fn default() -> Computer {
         Computer {
             ram: [0u8; 4096],
             cpu: Default::default(),
             keyboard: [false; 16]
         }
     }
}

fn combine(arr: &[u8]) -> u16 {
    let mut val: u16 = 0;
    for v in arr {
        val <<= 4;
        val += *v as u16;
    }
    val
}

impl Computer {
    fn write_hex_sprites(&mut self) {
        let sprites = [
            0xF0,0x90,0x90,0x90,0xF0, // 0
            0x20,0x60,0x20,0x20,0x70, // 1
            0xF0,0x10,0xF0,0x80,0xF0, // 2
            0xF0,0x10,0xF0,0x10,0xF0, // 3
            0x90,0x90,0xF0,0x10,0x10, // 4
            0xF0,0x80,0xF0,0x10,0xF0, // 5
            0xF0,0x80,0xF0,0x90,0xF0, // 6
            0xF0,0x10,0x20,0x40,0x40, // 7
            0xF0,0x90,0xF0,0x90,0xF0, // 8
            0xF0,0x90,0xF0,0x10,0xF0, // 9
            0xF0,0x90,0xF0,0x90,0x90, // A
            0xE0,0x90,0xE0,0x90,0xE0, // B
            0xF0,0x80,0x80,0x80,0xF0, // C
            0xE0,0x90,0x90,0x90,0xE0, // D
            0xF0,0x80,0xF0,0x80,0xF0, // E
            0xF0,0x80,0xF0,0x80,0x80  // F
        ];
        let len = sprites.len();
        for (i, val) in self.ram[0x000..len].iter_mut().enumerate() {
            *val = sprites[i];
        }
    }

    fn handle_key_event(&mut self, event: Event, keymap: &HashMap<Keycode, u8>) -> Option<u8> {
        let mut keycode = None;
        match event {
            Event::Quit { .. } => {
                std::process::exit(0);
            },
            Event::KeyDown {keycode: Some(key), ..} => {
                if key == Keycode::K {
                    std::process::exit(0);
                } else if keymap.contains_key(&key) {
                    keycode = Some(keymap.get(&key).unwrap().clone());
                    self.keyboard[keycode.unwrap() as usize] = true;
                }
            },
            Event::KeyUp {keycode: Some(key), ..} => {
                if keymap.contains_key(&key) {
                    keycode = Some(keymap.get(&key).unwrap().clone());
                    self.keyboard[keycode.unwrap() as usize] = false;
                }
            },
            _ => {}
        }

        return keycode;
    }


    fn ld_i_addr(&mut self, inst: &[u8; 4]) {
        let addr = combine(&inst[1..]);
        self.cpu.i = addr;
    }

    fn rnd_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let random_byte = rand::random::<u8>();
        let byte: u8 = kk & random_byte;
        self.cpu.v[inst[1] as usize] = byte;
    }

    fn sne_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let vx = self.cpu.v[inst[1] as usize];
        if kk != vx {
            self.cpu.pc += 2;
        }
    }

    fn se_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let vx = self.cpu.v[inst[1] as usize];
        if kk == vx {
            self.cpu.pc += 2;
        }
    }

    fn se_vx_vy(&mut self, inst: &[u8; 4]) {
        let vx = self.cpu.v[inst[1] as usize];
        let vy = self.cpu.v[inst[2] as usize];
        if vx == vy {
            self.cpu.pc += 2;
        }
    }

    fn drw_vx_vy_nibble(&mut self, inst: &[u8; 4]) {
        let screen_start: usize = self.ram.len() - 256 - 1;
        let x: u16 = self.cpu.v[inst[1] as usize] as u16;
        let y: u16 = self.cpu.v[inst[2] as usize] as u16;
        let n = inst[3] as u16;
        let mut sprite: Vec<u8> = Vec::new();
        sprite.extend_from_slice(&self.ram[(self.cpu.i as usize)..((self.cpu.i+(n as u16)) as usize)]);
        let offset: u16 = x % 8;
        let mut collided = false;
        for i in 0..n {
            let first_col = x / 8;
            let first_byte_i: usize = (((y + i).wrapping_mul(8)) + first_col) as usize + screen_start;

            let second_col = (x / 8 + 1) % 8;
            let second_byte_i: usize = (((y + i).wrapping_mul(8)) + second_col) as usize + screen_start;

            let byte: u8 = sprite[i as usize];
            let first_byte: u8=
                if offset == 8 { 0 } else { byte.wrapping_shr(offset as u32) };
            let second_byte: u8 =
                if offset == 0 { 0 } else { byte.wrapping_shl((8 - offset) as u32)};

            collided = collided || ((first_byte & self.ram[first_byte_i]) != 0);
            self.ram[first_byte_i] ^= first_byte;

            collided = collided || ((second_byte & self.ram[second_byte_i]) != 0);
            self.ram[second_byte_i] ^= second_byte;
        }
        self.cpu.v[0xf] = if collided { 1 } else { 0 };
    }

    fn add_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let x = inst[1] as usize;
        self.cpu.v[x] = self.cpu.v[x].wrapping_add(kk);
    }

    fn jmp_addr(&mut self, inst: &[u8; 4]) {
        self.cpu.pc = combine(&inst[1..]) as u16;
    }

    fn ld_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        self.cpu.v[inst[1] as usize] = kk;
    }

    fn call_addr(&mut self, inst: &[u8; 4]) {
        self.cpu.sp += 1;
        self.cpu.stack[self.cpu.sp as usize] = self.cpu.pc;
        self.cpu.pc = combine(&inst[1..]) as u16;
    }

    fn ret(&mut self) {
        self.cpu.pc = self.cpu.stack[self.cpu.sp as usize];
        self.cpu.sp -= 1;
    }

    fn and_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] &= self.cpu.v[y];
    }

    fn or_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] |= self.cpu.v[y];
    }

    fn xor_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] ^= self.cpu.v[y];
    }

    fn ld_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] = self.cpu.v[y];
    }

    fn add_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;

        // set vf if overflow occurs
        self.cpu.v[0xf] =
            if (self.cpu.v[x] as u16 + self.cpu.v[y] as u16) > 255 { 1 } else { 0 };

        self.cpu.v[x] = self.cpu.v[x].wrapping_add(self.cpu.v[y]);
    }

    fn sub_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;

        // set vf if vx > vy
        self.cpu.v[0xf] = if self.cpu.v[x] > self.cpu.v[y] { 1 } else { 0 };

        self.cpu.v[x] = self.cpu.v[x].wrapping_sub(self.cpu.v[y]);
    }

    fn shr_vx(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;

        // set vf if vx is odd
        self.cpu.v[0xf] = self.cpu.v[x] & 1;

        self.cpu.v[x] >>= 1;
    }

    fn shl_vx(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;

        // set vf if high order bit of vx is 1
        self.cpu.v[0xf] = self.cpu.v[x] & 0x80;

        self.cpu.v[x] <<= 1;
    }

    fn subn_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;

        // set vf if vx > vy
        self.cpu.v[0xf] = if self.cpu.v[y] > self.cpu.v[x] { 1 } else { 0 };

        self.cpu.v[x] = self.cpu.v[y].wrapping_sub(self.cpu.v[x]);
    }

    fn add_i_vx(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        self.cpu.i = self.cpu.i.wrapping_add(self.cpu.v[x] as u16);
    }

    fn ld_vx_k(&mut self, inst: &[u8; 4], keymap: &HashMap<Keycode, u8>,
               event_pump: &mut EventPump) {
        let keycode: u8;
        loop {
            let result = event_pump.poll_event();
            match result {
                Some(event) => {
                    let event_key = self.handle_key_event(event, keymap);
                    match event_key {
                        Some(key) => {
                            keycode = key;
                            break;
                        },
                        None => continue
                    }
                }
                None => continue
            }
        }
        if keycode >= 16 {
            error!("Something went horribly, horribly wrong and I'm so sorry\n");
            panic!("Something went horribly, horribly wrong and I'm so sorry\n");
        }
        self.cpu.v[inst[1] as usize] = keycode;
    }

    fn ld_i_vx(&mut self, inst: &[u8; 4]) {
        for i in 0..(inst[1] + 1) {
            self.ram[(self.cpu.i + i as u16) as usize] = self.cpu.v[i as usize];
        }
    }

    fn ld_vx_i(&mut self, inst: &[u8; 4]) {
        for i in 0..(inst[1] + 1) {
            self.cpu.v[i as usize] = self.ram[(self.cpu.i + i as u16) as usize];
        }
    }

    fn cls(&mut self) {
        let offset = self.ram.len() - 256 - 1;
        let screen = &mut self.ram[offset..];
        for v in screen.iter_mut() {
            *v = 0;
        }
    }

    fn ls_b_vx(&mut self, inst: &[u8; 4]) {
        let vx = self.cpu.v[inst[1] as usize];
        let i = self.cpu.i as usize;
        self.ram[i] = vx / 100;
        self.ram[i + 1] = (vx % 100) / 10;
        self.ram[i + 2] = vx % 10;
    }

    fn lf_f_vx(&mut self, inst: &[u8; 4]) {
        self.cpu.i = (self.cpu.v[inst[1] as usize] * 5) as u16;
    }

    fn jp_v0_addr(&mut self, inst: &[u8; 4]) {
        let addr = combine(&inst[1..]);
        self.cpu.pc = addr + self.cpu.v[0] as u16;
    }

    fn ld_vx_dt(&mut self, inst: &[u8; 4]) {
        self.cpu.v[inst[1] as usize] = self.cpu.dt;
    }

    fn ld_dt_vx(&mut self, inst: &[u8; 4]) {
        self.cpu.dt = self.cpu.v[inst[1] as usize];
    }

    fn ld_st_vx(&mut self, inst: &[u8; 4]) {
        self.cpu.st = self.cpu.v[inst[1] as usize];
    }

    fn skp_vx(&mut self, inst: &[u8; 4]) {
        let vx = self.cpu.v[inst[1] as usize];
        if self.keyboard[vx as usize] {
            self.cpu.pc += 2;
        }
    }

    fn sknp_vx(&mut self, inst: &[u8; 4]) {
        let vx = self.cpu.v[inst[1] as usize];
        if !self.keyboard[vx as usize] {
            self.cpu.pc += 2;
        }
    }
}

fn draw_screen_sdl(screen: &[u8], renderer: &mut Renderer) { 
    for row in 0..32 {
        for col in 0..8 {
            let byte = screen[(row * 8) + col];
            for bit in 0..8 {
                if ((byte >> bit) & 1) != 0 {
                    renderer.set_draw_color(ON_COLOR);
                } else {
                    renderer.set_draw_color(OFF_COLOR);
                }

                let x: i32 = ((col * 8 + 7 - bit) * X_SCALE as usize) as i32;
                let y: i32 = (row * Y_SCALE as usize) as i32;

                renderer.fill_rect(Rect::new(x, y, X_SCALE, Y_SCALE)).unwrap();
            }
        }
    }
    renderer.present();
}

fn unimplemented_panic(inst: &[u8; 4]) -> ! {
    error!("unimplemented instruction: {:x}{:x}{:x}{:x}\n",
            inst[0], inst[1], inst[2], inst[3]);
    panic!("unimplemented instruction: {:x}{:x}{:x}{:x}\n",
            inst[0], inst[1], inst[2], inst[3]);
}

fn main() {
    let keymap: HashMap<Keycode, u8> =
        [(Keycode::X,    0x0),
         (Keycode::Num1, 0x1),
         (Keycode::Num2, 0x2),
         (Keycode::Num3, 0x3),
         (Keycode::Q,    0x4),
         (Keycode::W,    0x5),
         (Keycode::E,    0x6),
         (Keycode::A,    0x7),
         (Keycode::S,    0x8),
         (Keycode::D,    0x9),
         (Keycode::Z,    0xa),
         (Keycode::C,    0xb),
         (Keycode::Num4, 0xc),
         (Keycode::R,    0xd),
         (Keycode::F,    0xe),
         (Keycode::V,    0xf)]
         .iter().cloned().collect();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window =
        video_subsystem.window("Chap8 - Chip8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    renderer.set_draw_color(Color::RGB(255, 0, 0));
    renderer.clear();
    renderer.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),  // mono
        samples: None       // default sample size
    };

    let buzzing = Arc::new(Mutex::new(false));

    let device = {
        let buzzing = buzzing.clone();
        audio_subsystem.open_playback(None, &desired_spec, move |spec| {
            // initialize the audio callback
            let buzzing = buzzing.lock().unwrap();

            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: if *buzzing { 0.25 } else { 0.0 } 
            }
        }).unwrap()
    };
    device.resume();

    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    // log_panics::init();

    let mut computer: Computer = Default::default();
    computer.cpu.pc = 0x200;
    computer.write_hex_sprites();

    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();
    let cycles_per_seconds = env::args().nth(2).unwrap_or(DEFAULT_CYCLES_PER_SECOND.to_string()).parse::<f64>().unwrap();

    {
        let end: usize = 0x200 + f.metadata().unwrap().len() as usize;
        let mut slice = &mut computer.ram[0x200..end];
        f.read_exact(slice).unwrap();
    }

    let delay_timer = timer::Timer::new();
    let (st, dt) = (Arc::new(Mutex::new(60u8)), Arc::new(Mutex::new(0u8)));
    let _delay_guard = {
        let (st, dt) = (st.clone(), dt.clone());
        delay_timer.schedule_repeating(chrono::Duration::nanoseconds(16666667), move || {
            {
                let mut st = st.lock().unwrap();
                if *st > 0 {
                    *st = *st - 1;
                }
            };
            {
                let mut dt = dt.lock().unwrap();
                if *dt > 0 {
                    *dt = *dt - 1;
                }
            };
        })
    };

    let cycle_timer = timer::Timer::new();
    let (tx, rx) = channel();
    let nanoseconds_per_cycle = (1000000000.0f64 / cycles_per_seconds) as i64;
    let _cycle_guard = cycle_timer.schedule_repeating(chrono::Duration::nanoseconds(nanoseconds_per_cycle), move || {
        tx.send(()).unwrap();
    });

    // thread::spawn(move|| {
    //     tx.send(()).unwrap();
    // });
    let offset = computer.ram.len() - 256 - 1;

    'main: loop {
        let _ = rx.recv().unwrap();

        for event in event_pump.poll_iter() {
            computer.handle_key_event(event, &keymap);
        }

        let mut should_inc = true;

        {
            let st = st.lock().unwrap();
            computer.cpu.st = *st;
        }
        {
            let dt = dt.lock().unwrap();
            computer.cpu.dt = *dt;
        }

        

        let inst: [u8; 4] = {
            let inst0 = computer.ram[computer.cpu.pc as usize];
            let inst1 = computer.ram[(computer.cpu.pc + 1) as usize];

            let tet0 = inst0 >> 4;
            let tet1 = 0x0f & inst0;
            let tet2 = inst1 >> 4;
            let tet3 = 0x0f & inst1;

            [tet0, tet1, tet2, tet3]
        };

        let inst_name: &str;

        match inst[0] {
            0x0 => {
                match inst[3] {
                    0x0 => {
                        inst_name = "cls";
                        computer.cls();
                    }
                    0xe => {
                        inst_name = "ret";
                        computer.ret();
                    },
                    _ => {
                        inst_name = "INVALID";
                    }
                }
            },
            0x1 => {
                inst_name = "jmp_addr";
                computer.jmp_addr(&inst);
                should_inc = false;
            },
            0x2 => {
                inst_name = "call_addr";
                computer.call_addr(&inst);
                should_inc = false;
            },
            0x3 => {
                inst_name = "se_vx_byte";
                computer.se_vx_byte(&inst);
            },
            0x4 => {
                inst_name = "sne_vx_byte";
                computer.sne_vx_byte(&inst);
            },
            0x5 => {
                inst_name = "se_vx_vy";
                computer.se_vx_vy(&inst);
            },
            0x6 => {
                inst_name = "ld_vx_byte";
                computer.ld_vx_byte(&inst);
            },
            0x7 => {
                inst_name = "add_vx_byte";
                computer.add_vx_byte(&inst);
            },
            0x8 => {
                match inst[3] {
                    0x0 => {
                        inst_name = "ld_vx_vy";
                        computer.ld_vx_vy(&inst);
                    },
                    0x1 => {
                        inst_name = "or_vx_vy";
                        computer.or_vx_vy(&inst);
                    },
                    0x2 => {
                        inst_name = "and_vx_vy";
                        computer.and_vx_vy(&inst);
                    },
                    0x3 => {
                        inst_name = "xor_vx_vy";
                        computer.xor_vx_vy(&inst);
                    },
                    0x4 => {
                        inst_name = "add_vx_vy";
                        computer.add_vx_vy(&inst);
                    },
                    0x5 => {
                        inst_name = "sub_vx_vy";
                        computer.sub_vx_vy(&inst);
                    },
                    0x6 => {
                        inst_name = "shr_vx";
                        computer.shr_vx(&inst);
                    },
                    0x7 => {
                        inst_name = "subn_vx_vy";
                        computer.subn_vx_vy(&inst);
                    },
                    0xe => {
                        inst_name = "shl_vx";
                        computer.shl_vx(&inst);
                    },
                    _ => unimplemented_panic(&inst)
                }
            },
            0xa => {
                inst_name = "ld_i_addr";
                computer.ld_i_addr(&inst);
            },
            0xb => {
                inst_name = "jp_v0_addr";
                computer.jp_v0_addr(&inst);
                should_inc = false;
            }
            0xc => {
                inst_name = "rnd_vx_byte";
                computer.rnd_vx_byte(&inst);
            },
            0xd => {
                inst_name = "drw_vx_vy_nibble";
                computer.drw_vx_vy_nibble(&inst);
                let screen = &computer.ram[offset..];
                draw_screen_sdl(screen, &mut renderer);

            },
            0xe => {
                match combine(&inst[2..]) {
                    0x9e => {
                        inst_name = "skp_vx";
                        computer.skp_vx(&inst);
                    },
                    0xa1 => {
                        inst_name = "sknp_vx";
                        computer.sknp_vx(&inst);
                    },
                    _=> unimplemented_panic(&inst)
                }
            },
            0xf => {
                match combine(&inst[2..]) {
                    0x07 => {
                        inst_name = "ld_vx_dt";
                        computer.ld_vx_dt(&inst);
                    },
                    0x0a => {
                        inst_name = "ld_vx_k";
                        computer.ld_vx_k(&inst, &keymap, &mut event_pump);
                    },
                    0x15 => {
                        inst_name = "ld_dt_vx";
                        computer.ld_dt_vx(&inst);
                        {
                            let mut dt = dt.lock().unwrap();
                            *dt = computer.cpu.dt;
                        }
                        
                    },
                    0x18 => {
                        inst_name = "ld_st_vx";
                        computer.ld_st_vx(&inst);
                        {
                            let mut st = st.lock().unwrap();
                            *st = computer.cpu.st;
                        }
                        
                    },
                    0x1e => {
                        inst_name = "add_i_vx";
                        computer.add_i_vx(&inst);
                    },
                    0x29 => {
                        inst_name = "lf_f_vx";
                        computer.lf_f_vx(&inst);
                    },
                    0x33 => {
                        inst_name = "ls_b_vx";
                        computer.ls_b_vx(&inst);
                    }
                    0x55 => {
                        inst_name = "ld_i_vx";
                        computer.ld_i_vx(&inst);
                    },
                    0x65 => {
                        inst_name = "ld_vx_i";
                        computer.ld_vx_i(&inst);
                    },
                    _ => unimplemented_panic(&inst)
                }
            },
            _ => unimplemented_panic(&inst)
        }
        debug!("inst: ");
        for x in &inst {
            debug!("{:x}", x);
        }
        debug!(" ({})\n", inst_name);

        if should_inc {
            computer.cpu.pc += 2;
        }

        debug!("{:?}\n", computer.cpu);
        {
            let mut buzzing = buzzing.lock().unwrap();
            *buzzing = computer.cpu.st != 0;
        }
        
    }
}

#[test]
fn combine_test1() {
    let inst = [0x1, 0x2, 0x3];
    let combo = combine(&inst);
    assert!(0x123 == combo);
}

#[test]
fn combine_test2() {
    let inst = [0x3];
    let combo = combine(&inst);
    assert!(0x3 == combo);
}

#[test]
fn combine_test3() {
    let inst = [0x1, 0x2, 0x3, 0x4];
    let combo = combine(&inst);
    assert!(0x1234 == combo);
}
