#![feature(proc_macro)]

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

const OFF_COLOR: ansi_term::Colour = RGB(153, 102, 0);
const ON_COLOR: ansi_term::Colour = RGB(255, 204, 0);
const OFF_PIXEL: &'static str = "...";
const ON_PIXEL: &'static str = "###";

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
}


fn draw_screen(screen: &[u8]) {
    for i in 0..32 {
        for j in 0..8 {
            let byte = screen[(i * 8) + j];
            for k in 0..8 {
                if ((byte >> k) & 1) != 0 {
                    print!("{}", OFF_COLOR.on(ON_COLOR).paint(ON_PIXEL));
                } else {
                    print!("{}", ON_COLOR.on(OFF_COLOR).paint(OFF_PIXEL));
                }
            }
        }
        println!("");
    }
    println!("");
}

// cargo run -- ./games/TANK

fn main() {
    let mut computer: Computer = Default::default();
    computer.cpu.pc = 0x200;
    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();

    {
        let end: usize = 0x200 + f.metadata().unwrap().len() as usize;
        let mut slice = &mut computer.ram[0x200..end];
        f.read_exact(slice).unwrap();
    }

    let offset = computer.ram.len() - 256 - 1;

    loop {
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
            0x3 => {
                inst_name = "set_vx_byte";
                computer.se_vx_byte(&inst);
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
                draw_screen(&computer.ram[offset..]);
            },
            0x7 => {
                inst_name = "add_vx_byte";
                computer.add_vx_byte(&inst);
            },
            0x1 => {
                inst_name = "jmp_addr";
                computer.jmp_addr(&inst);
                should_inc = false;
            },
            0x6 => {
                inst_name = "ld_vx_byte";
                computer.ld_vx_byte(&inst);
            }
            _ => panic!("unimplemented instruction: {:x}", inst[0])
        }
        // print!("inst: ");
        // for x in &inst {
        //     print!("{:x}", x);
        // }
        // println!(" ({})", inst_name);

        if should_inc {
            computer.cpu.pc += 2;
        }

        // println!("{:?}", computer.cpu);

        // step on newline
        // let mut input = String::new();
        // io::stdin().read_line(&mut input).unwrap();
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
