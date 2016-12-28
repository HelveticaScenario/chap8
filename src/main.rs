#![feature(proc_macro, core)]

extern crate rustbox;

#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate serde_derive;

extern crate rand;
extern crate serde;
extern crate serde_yaml;

extern crate ansi_term;

use std::fs::File;
use std::io::Read;
use std::io;
use std::env;
use std::fmt;
use ansi_term::Colour::RGB;

use std::time::Duration;

use rustbox::{Color, RustBox, OutputMode};
use rustbox::Color::Byte;
use rustbox::Key;

const OFF_COLOR: ansi_term::Colour = RGB(153, 102, 0);
const ON_COLOR: ansi_term::Colour = RGB(255, 204, 0);
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


// #[derive(Default)]
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

// fn expand(byte: u8) -> Vec<bool> {
//     let mut ret = Vec::new();
// }

// fn flip_bit(byte: u8, addr: u8) -> (u8, bool) {
//     let old = get_bit(byte, addr);
//     byte ^= 1 << addr;
//     let new = get_bit(byte, addr);
    
//     return (byte, old == true && new == false);
// }

// fn get_bit(byte: u8, addr: u8) -> bool {
//     return ((byte >> addr) & 1) != 0;
// }

impl Computer {
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

    fn se_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let vx = self.cpu.v[inst[1] as usize];
        if kk == vx {
            self.cpu.pc += 2;
        }
    }

    fn sne_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        let vx = self.cpu.v[inst[1] as usize];
        if kk != vx {
            self.cpu.pc += 2;
        }
    }


    fn drw_vx_vy_nibble(&mut self, inst: &[u8; 4]) {
        let screen_start: usize = self.ram.len() - 256 - 1;
        let x = self.cpu.v[inst[1] as usize];
        let y = self.cpu.v[inst[2] as usize];
        let n = inst[3] as u8;
        let mut sprite: Vec<u8> = Vec::new();
        // sprite.reserve_exact(n as usize);
        sprite.extend_from_slice(&self.ram[(self.cpu.i as usize)..((self.cpu.i+(n as u16)) as usize)]);
        let offset: u8 = x % 8;
        let mut collided = false;
        for i in 0..n {
            let first_byte: usize = (((y + i) * 8) + (x / 8)) as usize + screen_start;
            let second_byte: usize = ((y + i) * 8 + ((x + 8) % 64) / 8) as usize + screen_start;

            let mut shift: u8 = sprite[i as usize] >> offset;
            collided = collided || ((shift & self.ram[first_byte]) != 0);
            self.ram[first_byte] ^= shift;

            shift = sprite[i as usize] << (7 - offset);
            collided = collided || ((shift & self.ram[second_byte]) != 0);
            self.ram[second_byte] ^= shift;
        }
        self.cpu.v[0xf] = if collided { 1 } else { 0 };
    }

    fn add_vx_byte(&mut self, inst: &[u8; 4]) {
        let kk = combine(&inst[2..]) as u8;
        self.cpu.v[inst[1] as usize] += kk;
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

    fn ret(&mut self, inst: &[u8; 4]) {
        self.cpu.pc = self.cpu.stack[self.cpu.sp as usize];
        self.cpu.sp -= 1;
    }

    fn and_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] &= self.cpu.v[y];
    }

    fn ld_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] = self.cpu.v[y];
    }

    fn sub_vx_vy(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        let y = inst[2] as usize;
        self.cpu.v[x] -= self.cpu.v[y];
    }

    fn add_i_vx(&mut self, inst: &[u8; 4]) {
        let x = inst[1] as usize;
        self.cpu.i += self.cpu.v[x] as u16;
    }
}


fn draw_screen(screen: &[u8]) {

    for y in 0..32 {
        for x in 0..8 {
            let byte = screen[(y * 8) + x];
            for bit in 0..8 {
                for i in 0..PIXEL_WIDTH {
                    if ((byte >> bit) & 1) != 0 {
                        print!("{}", OFF_COLOR.on(ON_COLOR).paint(ON_PIXEL.to_string()));
                    } else {
                        print!("{}", ON_COLOR.on(OFF_COLOR).paint(OFF_PIXEL.to_string()));
                    }
                }
            }
        }
        println!("");
    }
    println!("");
}

fn draw_screen_rustbox(screen: &[u8], rustbox: &RustBox) { 
    for y in 0..32 {
        for x in 0..8 {
            let byte = screen[(y * 8) + x];
            for bit in 0..8 {
                for i in 0..PIXEL_WIDTH {
                    if ((byte >> bit) & 1) != 0 {
                        rustbox.print_char((((x * 8) + bit) * PIXEL_WIDTH) + i, y, rustbox::RB_NORMAL, ON_COLOR_BOX, ON_COLOR_BOX, ON_PIXEL);
                    } else {
                        rustbox.print_char((((x * 8) + bit) * PIXEL_WIDTH) + i, y, rustbox::RB_NORMAL, OFF_COLOR_BOX, OFF_COLOR_BOX, OFF_PIXEL);
                    }
                }
            }
        }
    }
    rustbox.present();
}

// cargo run -- ./games/TANK

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    debug!("A FILE HAPPENED :O :O :O ");

    let mut computer: Computer = Default::default();
    computer.cpu.pc = 0x200;
    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();

    {
        let end: usize = 0x200 + f.metadata().unwrap().len() as usize;
        let mut slice = &mut computer.ram[0x200..end];
        f.read_exact(slice).unwrap();
    }

    let offset = computer.ram.len() - 256 - 1;

    let mut rustbox = match RustBox::init(Default::default()) {
        Result::Ok(v) => v,
        Result::Err(e) => panic!("{}", e),
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
            Err(e) => panic!("{}", e),
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
                    0xe => {
                        inst_name = "ret";
                        computer.ret(&inst);
                    },
                    _ => {
                        error!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
                        panic!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
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
                    0x2 => {
                        inst_name = "and_vx_vy";
                        computer.and_vx_vy(&inst);
                    },
                    0x5 => {
                        inst_name = "sub_vx_vy";
                        computer.sub_vx_vy(&inst);
                    },
                    _ => {
                        error!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
                        panic!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
                    }
                }
            },
            0xa => {
                inst_name = "ld_i_addr";
                computer.ld_i_addr(&inst);
            },
            0xc => {
                inst_name = "rnd_vx_byte";
                computer.rnd_vx_byte(&inst);
            },
            0xd => {
                inst_name = "drw_vx_vy_nibble";
                computer.drw_vx_vy_nibble(&inst);
                let screen = &computer.ram[offset..];
                draw_screen_rustbox(screen, &rustbox);
                // draw_screen(screen);
            },
            0xf => {
                match combine(&inst[2..]) {
                    0x1e => {
                        inst_name = "add_i_vx";
                        computer.add_i_vx(&inst);
                    },
                    _ => {
                        error!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
                        panic!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                                inst[0], inst[1], inst[2], inst[3]);
                    }
                }
            },
            _ => {
                error!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                        inst[0], inst[1], inst[2], inst[3]);
                panic!("unimplemented instruction: {:x}{:x}{:x}{:x}",
                        inst[0], inst[1], inst[2], inst[3]);
            }
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
