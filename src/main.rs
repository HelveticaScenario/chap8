#![feature(proc_macro)]

extern crate rustbox;

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

use std::error::Error;
use std::default::Default;

use std::fs::File;
use std::io::Read;
use std::env;
use std::fmt;

use std::time::Duration;

use rustbox::{RustBox, OutputMode};
use rustbox::Color::Byte;
use rustbox::Key;

const OFF_COLOR_BOX: rustbox::Color = Byte(130u16);
const ON_COLOR_BOX: rustbox::Color = Byte(148u16);
const PIXEL_WIDTH: usize = 3;
const OFF_PIXEL: char = '.';
const ON_PIXEL: char = '#';

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
}

impl Default for Computer {
     fn default() -> Computer {
         Computer {
             ram: [0u8; 4096],
             cpu: Default::default(),
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
        let x = self.cpu.v[inst[1] as usize];
        let y = self.cpu.v[inst[2] as usize];
        let n = inst[3] as u8;
        let mut sprite: Vec<u8> = Vec::new();
        sprite.extend_from_slice(&self.ram[(self.cpu.i as usize)..((self.cpu.i+(n as u16)) as usize)]);
        let offset: u8 = x % 8;
        let mut collided = false;
        for i in 0..n {
            let first_byte_i: usize = (((y + i) * 8) + (x / 8)) as usize + screen_start;
            let second_byte_i: usize = ((y + i) * 8 + ((x + 8) % 64) / 8) as usize + screen_start;

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

    fn ld_vx_k(&mut self, inst: &[u8; 4], rustbox: &RustBox) {
        let key_char: Key;
        loop {
            match rustbox.poll_event(false) {
                Ok(rustbox::Event::KeyEvent(key)) => {
                    match key {
                        Key::Char('x') |
                        Key::Char('1') |
                        Key::Char('2') |
                        Key::Char('3') |
                        Key::Char('q') |
                        Key::Char('w') |
                        Key::Char('e') |
                        Key::Char('a') |
                        Key::Char('s') |
                        Key::Char('d') |
                        Key::Char('z') |
                        Key::Char('c') |
                        Key::Char('4') |
                        Key::Char('r') |
                        Key::Char('f') |
                        Key::Char('v') => { 
                            key_char = key;
                            break;
                        },
                        Key::Char('k') => {
                            panic!("you pressed k");
                        },
                        _ => { }
                    }
                },
                Err(e) => {
                    debug!("{}", e.description());
                    panic!("{}", e.description());
                },
                _ => { }
            };
        }
        let key_code: u8 = key_char_to_u8(key_char);
        if key_code >= 16 {
            error!("Something went horribly, horribly wrong and I'm so sorry\n");
            panic!("Something went horribly, horribly wrong and I'm so sorry\n");
        }
        self.cpu.v[inst[1] as usize] = key_code;
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
}

fn key_char_to_u8(key: Key) -> u8 {
    match key {
        Key::Char('x') => { 
            0
        }
        Key::Char('1') => { 
            1
        }
        Key::Char('2') => { 
            2
        }
        Key::Char('3') => { 
            3
        }
        Key::Char('q') => { 
            4
        }
        Key::Char('w') => { 
            5
        }
        Key::Char('e') => { 
            6
        }
        Key::Char('a') => { 
            7
        }
        Key::Char('s') => { 
            8
        }
        Key::Char('d') => { 
            9
        }
        Key::Char('z') => { 
            10
        }
        Key::Char('c') => { 
            11
        }
        Key::Char('4') => { 
            12
        }
        Key::Char('r') => { 
            13
        }
        Key::Char('f') => { 
            14
        }
        Key::Char('v') => { 
            15
        }
        _ => { 
            16
        }
    }
}

fn draw_screen_rustbox(screen: &[u8], rustbox: &RustBox) { 
    for y in 0..32 {
        for x in 0..8 {
            let byte = screen[(y * 8) + x];
            for bit in 0..8 {
                for i in 0..PIXEL_WIDTH {
                    if ((byte >> bit) & 1) != 0 {
                        rustbox.print_char(
                            (((x * 8) + (7 - bit)) * PIXEL_WIDTH) + i, y,
                            rustbox::RB_NORMAL, ON_COLOR_BOX, ON_COLOR_BOX, ON_PIXEL);
                    } else {
                        rustbox.print_char(
                            (((x * 8) + (7 - bit)) * PIXEL_WIDTH) + i, y,
                            rustbox::RB_NORMAL, OFF_COLOR_BOX, OFF_COLOR_BOX, OFF_PIXEL);
                    }
                }
            }
        }
    }
    rustbox.present();
}

fn unimplemented_panic(inst: &[u8; 4]) -> ! {
    error!("unimplemented instruction: {:x}{:x}{:x}{:x}\n",
            inst[0], inst[1], inst[2], inst[3]);
    panic!("unimplemented instruction: {:x}{:x}{:x}{:x}\n",
            inst[0], inst[1], inst[2], inst[3]);
}

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    log_panics::init();

    let mut computer: Computer = Default::default();
    computer.cpu.pc = 0x200;
    computer.write_hex_sprites();

    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();

    {
        let end: usize = 0x200 + f.metadata().unwrap().len() as usize;
        let mut slice = &mut computer.ram[0x200..end];
        f.read_exact(slice).unwrap();
    }

    let offset = computer.ram.len() - 256 - 1;

    let mut rustbox = match RustBox::init(Default::default()) {
        Result::Ok(v) => v,
        Result::Err(e) => panic!("{}\n", e),
    };
    rustbox.set_output_mode(OutputMode::EightBit);

    loop {
        match rustbox.peek_event(Duration::from_millis(1), false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    Key::Char('k') => { break; }
                    _ => { }
                }
            },
            Err(e) => panic!("{}\n", e),
            _ => { }
        }

        let mut should_inc = true;

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
                draw_screen_rustbox(screen, &rustbox);
            },
            // need exa1
            // need ex9e
            0xf => {
                match combine(&inst[2..]) {
                    0x07 => {
                        inst_name = "ld_vx_dt";
                        computer.ld_vx_dt(&inst, &rustbox);
                    },
                    0x0a => {
                        inst_name = "ld_vx_k";
                        computer.ld_vx_k(&inst, &rustbox);
                    },
                    // need 15
                    // need 18
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
