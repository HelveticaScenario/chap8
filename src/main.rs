use std::fs::File;
use std::env;

#[derive(Default)]
struct CPU {
    v: [u8; 16],
    i: u16,
    dt: u8,
    st: u8,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
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

impl Computer {

}

// cargo run -- ./games/TANK

fn main() {
    let mut computer : Computer = Default::default();
    computer.cpu.pc = 0x200;
    let mut f = File::open(env::args().nth(1).unwrap()).unwrap();
    {
        let mut slice = &mut computer.ram[0x200..];
        slice[0] = 55u8;
    }
    println!("{:?}", computer.ram[0x200]);
    

    
    println!("Hello, world!");
}
