#![feature(proc_macro)]

#[macro_use]
extern crate serde_derive;

extern crate rand;
extern crate serde;
extern crate serde_yaml;

use std::fs::File;
use std::io::Read;
use std::io;
use std::env;
use std::fmt;

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
        write!(f, "v : ");
        for v in &self.v {
            write!(f, "{:x}, ", v);
        }
        write!(f, "\n");

        write!(f, "i : {:x}\n", self.i);
        write!(f, "dt: {:x}\n", self.dt);
        write!(f, "st: {:x}\n", self.st);
        write!(f, "pc: {:x}\n", self.pc);
        write!(f, "sp: {:x}\n", self.sp);

        write!(f, "sk: ");
        for s in &self.stack {
            write!(f, "{:x}, ", s);
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
}

// cargo run -- ./games/TANK

fn main() {
    let mut computer: Computer = Default::default();
    computer.cpu.pc = 0x200;
    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();

    {
        let end: usize = 0x200 + f.metadata().unwrap().len() as usize;
        let mut slice = &mut computer.ram[0x200..end];
        f.read_exact(slice);
    }

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

        print!("inst: ");
        for x in &inst {
            print!("{:x}", x);
        }

        let mut inst_name = "";

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
            _ => panic!("unimplemented instruction: {:x}", inst[0])
        }

        println!(" ({})", inst_name);

        if should_inc {
            computer.cpu.pc += 2;
        }

        println!("{:?}", computer.cpu);

        // step on newline
        let mut input = String::new();
        io::stdin().read_line(&mut input);
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
